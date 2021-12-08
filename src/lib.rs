extern crate ladspa;
extern crate rustfft;
extern crate realfft;

use ladspa::{PluginDescriptor, PortDescriptor, Port, DefaultValue, Data, Plugin, PortConnection, HINT_LOGARITHMIC, HINT_TOGGLED};
use rustfft::{FftPlanner, num_complex::Complex32};
use realfft::{RealFftPlanner};
use std::default::Default;

struct Denoise {
    sample_rate: Data,
    planner: RealFftPlanner<f32>,
    input_buf: Vec<Vec<f32>>,
    buf: Vec<f32>,
}

const LENGTH: usize = 1024;

fn new_denoise(_: &PluginDescriptor, sample_rate: u64) -> Box<dyn Plugin + Send> {
    println!("Goldilocks denoiser initialized.");
    Box::new(Denoise {
        sample_rate: sample_rate as Data,
        planner: RealFftPlanner::<f32>::new(),
        input_buf: vec![vec![0.0; LENGTH], vec![0.0; LENGTH]],
        buf: Vec::new(),
    })
}

impl Plugin for Denoise {

    //fn activate(&mut self) {
    //}

    fn run<'a>(&mut self, sample_count: usize, ports: &[&'a PortConnection<'a>]) {
        let mut  input  = vec![ports[0].unwrap_audio().to_vec(), ports[1].unwrap_audio().to_vec()];
        let mut  output = vec![ports[2].unwrap_audio_mut(), ports[3].unwrap_audio_mut()];
        let mut output_r = vec![vec![0.0; LENGTH], vec![0.0; LENGTH]];
        // learn features not implemented yet
        let      learn  = ports[4].unwrap_control() > &0.5;
        let      learn_time = ports[5].unwrap_control() * self.sample_rate;
        let      noise_floor = &20.0 * &10.0f32.powf(ports[6].unwrap_control()/&20.0);

        let rfft = self.planner.plan_fft_forward(LENGTH);
        let mut c_out = vec![rfft.make_output_vec(), rfft.make_output_vec()];
        let irfft = self.planner.plan_fft_inverse(LENGTH);

        let s = 1.0/(LENGTH as f32);

        let step = sample_count.min(LENGTH);

        for (j, chan) in c_out.iter_mut().enumerate() {

            for n in 0..(LENGTH/step) {
                self.input_buf[j] = self.input_buf[j][step..LENGTH].to_vec();
                assert_eq!(self.input_buf[j].len(), LENGTH-step);
                self.input_buf[j].append(&mut input[j][n*step..(n+1)*step].to_vec());
                assert_eq!(self.input_buf[j].len(), LENGTH);

                //
                rfft.process(&mut self.input_buf[j], chan).unwrap();

                for (i, sample) in chan.iter_mut().enumerate() {
                    *sample = sample.scale(s);
                    let (mut mag, phase) = sample.to_polar();
                    if learn { self.buf[i] *= mag * 1.0/learn_time; }
                    if mag < noise_floor {mag = 0.0}
                    //*sample = Complex32::from_polar(mag, phase);
                }
                irfft.process(chan, &mut output_r[j]).unwrap();

                output[j][n*step..(n+1)*step].copy_from_slice(&output_r[j]);
                //assert_eq!(&output_r)
            }
        }
    }
}

#[no_mangle]
pub fn get_ladspa_descriptor(index: u64) -> Option<PluginDescriptor> {
    match index {
        0 => {
                Some(PluginDescriptor {
                    unique_id: 400,
                    label: "goldilocks",
                    properties: ladspa::PROP_NONE,
                    name: "Goldilocks FIR denosier",
                    maker: "Daniel Hill",
                    copyright: "MIT",
                    ports: vec![
                        Port {
                            name: "Left Audio In",
                            desc: PortDescriptor::AudioInput,
                            ..Default::default()
                        },
                        Port {
                            name: "Right Audio In",
                            desc: PortDescriptor::AudioInput,
                            ..Default::default()
                        },
                        Port {
                            name: "Left Audio Out",
                            desc: PortDescriptor::AudioOutput,
                            ..Default::default()
                        },
                        Port {
                            name: "Right Audio Out",
                            desc: PortDescriptor::AudioOutput,
                            ..Default::default()
                        },
                        Port {
                            name: "Learn",
                            desc: PortDescriptor::ControlInput,
                            default: Some(DefaultValue::Value0),
                            lower_bound: Some(0.0),
                            upper_bound: Some(1.0),
                            hint: Some(HINT_TOGGLED)
                        },
                        Port {
                            name: "Learn Window (seconds)",
                            desc: PortDescriptor::ControlInput,
                            default: Some(DefaultValue::Middle),
                            upper_bound: Some(30.0),
                            lower_bound: Some(0.1),
                            hint: None
                        },
                        Port {
                            name: "Noise Floor",
                            desc: PortDescriptor::ControlInput,
                            hint: Some(HINT_LOGARITHMIC),
                            default: Some(DefaultValue::Low),
                            upper_bound: Some(60.0),
                            lower_bound: Some(-96.0),
                        }
                    ],
                    new: new_denoise
            })
        },
        _ => None
    }
}
