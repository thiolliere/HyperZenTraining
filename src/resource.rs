use vulkano::command_buffer::AutoCommandBuffer;
use app_dirs::{AppInfo, app_root, AppDataType};

pub use graphics::Data as Graphics;
use std::io::Write;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;
use std::collections::HashMap;
pub use audio::Audio;

pub type PhysicWorld = ::nphysics::world::World<f32>;
pub struct Events(pub Vec<::winit::Event>);
pub type Benchmarks = Vec<::util::Benchmark>;

pub struct Activated(pub bool);

pub type ImGuiOption = Option<::imgui::ImGui>;

pub struct FpsCounter(pub usize);

#[derive(Deserialize, Serialize)]
pub struct Save {
    mouse_sensibility: f32,
    scores: HashMap<usize, Score>,
}

#[derive(Deserialize, Serialize)]
pub struct Score {
    pub bests: Vec<Duration>,
    pub lasts: Vec<Duration>,
}

impl Score {
    fn new() -> Self {
        Score {
            bests: vec![],
            lasts: vec![],
        }
    }
    fn insert(&mut self, duration: Duration) {
        self.bests.push(duration);
        self.bests.sort();
        self.bests.truncate(10);

        self.lasts.insert(0, duration);
        self.lasts.truncate(10);
    }
}

const APP_INFO: AppInfo = AppInfo { name: "pepe", author: "thiolliere" };
const FILENAME: &str = "save.ron";

lazy_static! {
    static ref SAVE_PATH: PathBuf = {
        let mut path = app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
        path.push(FILENAME);
        path
    };
}

impl Save {
    pub fn new() -> Self {
        File::open(SAVE_PATH.as_path()).ok()
            .and_then(|file| ::ron::de::from_reader(file).ok())
            .unwrap_or(Save {
                mouse_sensibility: ::CONFIG.mouse_sensibility,
                scores: HashMap::new(),
            })
    }

    #[inline]
    pub fn mouse_sensibility(&self) -> f32 {
        self.mouse_sensibility
    }

    pub fn insert_score(&mut self, level: usize, score: Duration) {
        self.scores.entry(level).or_insert(Score::new()).insert(score);
        self.save();
    }

    pub fn score(&self, level: usize) -> Option<&Score> {
        self.scores.get(&level)
    }

    /// Do nothing if sensibility hasn't changed
    pub fn set_mouse_sensibility(&mut self, mouse_sensibility: f32) {
        if self.mouse_sensibility != mouse_sensibility {
            self.mouse_sensibility = mouse_sensibility;
            self.save();
        }
    }

    pub fn save(&self) {
        let string = ::ron::ser::to_string(&self).unwrap();
        let mut file = File::create(SAVE_PATH.as_path()).unwrap();
        file.write_all(string.as_bytes()).unwrap();
    }
}

pub struct UpdateTime(pub f32);

pub struct GameDuration(pub Duration);

pub struct Rendering {
    pub image_num: Option<usize>,
    pub command_buffer: Option<AutoCommandBuffer>,
    pub second_command_buffer: Option<AutoCommandBuffer>,
    pub size_points: Option<(u32, u32)>,
    pub size_pixels: Option<(u32, u32)>,
}

impl Rendering {
    pub fn new() -> Self {
        Rendering {
            image_num: None,
            command_buffer: None,
            second_command_buffer: None,
            size_points: None,
            size_pixels: None,
        }
    }
}

pub struct DebugMode(pub bool);

pub struct DepthCoef(pub f32);

pub struct PlayerControl {
    pub directions: Vec<::util::Direction>,
    pub pointer: [f32; 2],
}

impl PlayerControl {
    pub fn new() -> Self {
        PlayerControl {
            directions: vec![],
            pointer: [0.0, 0.0],
        }
    }
}

pub struct LevelActions(pub Vec<LevelAction>);

#[derive(Clone)]
pub enum LevelAction {
    Next,
    Reset,
    ReturnHall,
    Level(usize),
}

pub enum Maze {
    Maze2D(::maze::Maze<::na::U2>),
    Maze3D(::maze::Maze<::na::U3>),
}

impl Maze {
    pub fn find_path(
        &self,
        pos: ::na::Vector3<f32>,
        goal: ::na::Vector3<f32>,
    ) -> Option<Vec<::na::Vector3<f32>>> {
        match *self {
            Maze::Maze2D(ref maze) => maze.find_path(pos, goal),
            Maze::Maze3D(ref maze) => maze.find_path(pos, goal),
        }
    }

    pub fn is_3d(&self) -> bool {
        match *self {
            Maze::Maze2D(_) => false,
            Maze::Maze3D(_) => true,
        }
    }
}

pub struct MenuState {
    pub pause: bool,
    pub mouse_sensibility_input: f32,
    pub continue_button: bool,
    pub reset_button: bool,
    pub return_hall_button: bool,
    pub quit_button: bool,
    pub levels_button: [bool; 16],
}

impl MenuState {
    pub fn new(save: &Save) -> Self {
        MenuState {
            pause: false,
            mouse_sensibility_input: save.mouse_sensibility(),
            continue_button: false,
            reset_button: false,
            return_hall_button: false,
            quit_button: false,
            levels_button: [false; 16],
        }
    }

    pub fn build_ui(&mut self, ui: &::imgui::Ui) {
        let (width, height) = ui.imgui().display_size();
        let button_size = (76.0, 30.0);
        if self.pause {
            ui.window(im_str!("Pause"))
                .collapsible(false)
                .size((::CONFIG.menu_width, ::CONFIG.menu_height), ::imgui::ImGuiCond::Always)
                .position((width/2.0-::CONFIG.menu_width/2.0, height/2.0-::CONFIG.menu_height/2.0), ::imgui::ImGuiCond::Always)
                .resizable(false)
                .movable(false)
                .build(|| {
                    self.continue_button = ui.button(im_str!("Continue"), button_size);
                    ui.separator();
                    ui.text(im_str!("Levels :"));
                    ui.separator();
                    self.return_hall_button = ui.button(im_str!("Return to hall"), button_size);
                    ui.separator();
                    ui.text(im_str!("Settings :"));
                    ui.separator();
                    ui.input_float(im_str!("Mouse sensibility"), &mut self.mouse_sensibility_input).build();
                    self.reset_button = ui.button(im_str!("Reset"), button_size);
                    ui.separator();
                    self.quit_button = ui.button(im_str!("Quit"), button_size);
                });
        } else {
            // TODO: tutorial
            // ui.window(im_str!("Tutorial"))
            //     .collapsible(false)
            //     .always_auto_resize(true)
            //     .resizable(false)
            //     .movable(false)
            //     .build(|| {
            //         ui.text(im_str!("Press right click :-)"));
            //     });
        }
    }
}
