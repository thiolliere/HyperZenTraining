use std::fs::File;
use std::path::PathBuf;

use rodio::decoder::Decoder;
use rodio::Source;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Sound {
    Shoot,
    // BouncerBounce,
    // AvoiderBounce,
}

pub struct Audio {
    endpoint: ::rodio::Endpoint,
    spatial_sinks: Vec<::rodio::SpatialSink>,
    sounds: Vec<::rodio::source::Buffered<Decoder<File>>>,
    left_ear: [f32; 3],
    right_ear: [f32; 3],
}

impl Audio {
    pub fn init() -> Self {
        let sound_filenames = [
            ::CONFIG.player_shoot_sound.clone(),
        ];

        let mut sounds = vec![];
        for filename in sound_filenames.iter() {
            let mut path = PathBuf::from(::CONFIG.sound_dir.clone());
            path.push(filename);
            let file = File::open(path).unwrap();
            let sound = Decoder::new(file).unwrap().buffered();
            sounds.push(sound);
        }

        Audio {
            endpoint: ::rodio::get_default_endpoint().unwrap(),
            spatial_sinks: vec![],
            left_ear: [::std::f32::NAN; 3],
            right_ear: [::std::f32::NAN; 3],
            sounds,
        }
    }

    pub fn play(&mut self, sound: Sound, pos: [f32; 3]) {
        let spatial_sink = ::rodio::SpatialSink::new(
            &self.endpoint,
            pos,
            self.left_ear,
            self.right_ear,
        );
        spatial_sink.append(self.sounds[sound as usize].clone());
        self.spatial_sinks.push(spatial_sink);
    }

    pub fn set_emitter(&mut self, position: ::na::Vector3<f32>, aim: ::na::UnitQuaternion<f32>) {
        let local_left_ear = ::na::Point3::new(0.0, - ::CONFIG.ear_distance/2.0, 0.0);
        let local_right_ear = ::na::Point3::new(0.0, ::CONFIG.ear_distance/2.0, 0.0);

        let world_trans = ::na::Isometry::from_parts(
            ::na::Translation::from_vector(position),
            aim,
        );

        let left_ear = world_trans * local_left_ear;
        let right_ear = world_trans * local_right_ear;

        self.left_ear = left_ear.coords.into();
        self.right_ear = right_ear.coords.into();
        for spatial_sink in &mut self.spatial_sinks {
            spatial_sink.set_left_ear_position(self.left_ear);
            spatial_sink.set_right_ear_position(self.right_ear);
        }
    }
}