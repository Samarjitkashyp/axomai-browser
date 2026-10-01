//! Web Audio API Graph & Synthesizer Engine for Axomai Browser.
//! Implements W3C Web Audio API (AudioContext, AudioNode graph, Oscillator, Gain, BiquadFilter, Analyser).

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioContextState {
    Suspended,
    Running,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillatorType {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiquadFilterType {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
    Peaking,
    Allpass,
}

#[derive(Debug, Clone)]
pub struct AudioParam {
    pub value: f32,
    pub default_value: f32,
    pub min_value: f32,
    pub max_value: f32,
}

impl AudioParam {
    pub fn new(val: f32, min: f32, max: f32) -> Self {
        AudioParam {
            value: val,
            default_value: val,
            min_value: min,
            max_value: max,
        }
    }

    pub fn set_value(&mut self, val: f32) {
        self.value = val.clamp(self.min_value, self.max_value);
    }
}

#[derive(Debug, Clone)]
pub enum AudioNodeKind {
    Destination,
    Gain { gain: AudioParam },
    Oscillator { osc_type: OscillatorType, frequency: AudioParam, detune: AudioParam },
    BiquadFilter { filter_type: BiquadFilterType, frequency: AudioParam, q: AudioParam, gain: AudioParam },
    Analyser { fft_size: usize, min_decibels: f32, max_decibels: f32 },
}

#[derive(Debug, Clone)]
pub struct AudioNode {
    pub id: u32,
    pub kind: AudioNodeKind,
    pub connected_to: Vec<u32>,
}

pub struct AudioContext {
    pub sample_rate: u32,
    pub current_time: f64,
    pub state: AudioContextState,
    pub destination_id: u32,
    next_node_id: u32,
    pub nodes: HashMap<u32, AudioNode>,
}

impl AudioContext {
    pub fn new(sample_rate: u32) -> Self {
        let mut ctx = AudioContext {
            sample_rate,
            current_time: 0.0,
            state: AudioContextState::Running,
            destination_id: 1,
            next_node_id: 2,
            nodes: HashMap::new(),
        };

        // Destination node (ID 1)
        ctx.nodes.insert(1, AudioNode {
            id: 1,
            kind: AudioNodeKind::Destination,
            connected_to: Vec::new(),
        });

        ctx
    }

    pub fn create_gain(&mut self, initial_gain: f32) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(id, AudioNode {
            id,
            kind: AudioNodeKind::Gain {
                gain: AudioParam::new(initial_gain, 0.0, 10.0),
            },
            connected_to: Vec::new(),
        });
        id
    }

    pub fn create_oscillator(&mut self, osc_type: OscillatorType, freq: f32) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(id, AudioNode {
            id,
            kind: AudioNodeKind::Oscillator {
                osc_type,
                frequency: AudioParam::new(freq, 0.0, 24000.0),
                detune: AudioParam::new(0.0, -153600.0, 153600.0),
            },
            connected_to: Vec::new(),
        });
        id
    }

    pub fn create_biquad_filter(&mut self, filter_type: BiquadFilterType, freq: f32) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(id, AudioNode {
            id,
            kind: AudioNodeKind::BiquadFilter {
                filter_type,
                frequency: AudioParam::new(freq, 10.0, 24000.0),
                q: AudioParam::new(1.0, 0.0001, 1000.0),
                gain: AudioParam::new(0.0, -40.0, 40.0),
            },
            connected_to: Vec::new(),
        });
        id
    }

    pub fn create_analyser(&mut self, fft_size: usize) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(id, AudioNode {
            id,
            kind: AudioNodeKind::Analyser {
                fft_size,
                min_decibels: -100.0,
                max_decibels: -30.0,
            },
            connected_to: Vec::new(),
        });
        id
    }

    pub fn connect(&mut self, source_id: u32, target_id: u32) -> Result<(), String> {
        if !self.nodes.contains_key(&source_id) {
            return Err(format!("Source node {} does not exist", source_id));
        }
        if !self.nodes.contains_key(&target_id) {
            return Err(format!("Target node {} does not exist", target_id));
        }

        if let Some(source) = self.nodes.get_mut(&source_id) {
            if !source.connected_to.contains(&target_id) {
                source.connected_to.push(target_id);
            }
        }
        Ok(())
    }

    pub fn suspend(&mut self) {
        self.state = AudioContextState::Suspended;
    }

    pub fn resume(&mut self) {
        self.state = AudioContextState::Running;
    }

    pub fn close(&mut self) {
        self.state = AudioContextState::Closed;
    }
}
