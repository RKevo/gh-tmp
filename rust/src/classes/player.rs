//! Since we can't (yet) step physics manually. The plan is to experiment with godot's built-ins
//! for now and swap out for a manually managed physics frontend later.

use std::hint::cold_path;

use godot::{
	classes::{
		Camera3D, CharacterBody3D, CollisionShape3D, Engine, ICharacterBody3D, Input,
		class_macros::private::virtuals::{
			Xrvrs::Gd,
			ZipReader::{Vector2, Vector3},
		},
	},
	global::Key,
	obj::{Base, Singleton, WithBaseField},
	register::{GodotClass, godot_api},
};

use crate::Real;

type Maybe<T> = Option<Gd<T>>;

const BASE_MAX_SPEED: f64 = 3.0;
const BASE_ACCEL: f64 = 1.0;
const BASE_DECEL: f64 = 2.0;
const PRESERVED_TICKS: u64 = 30;
const BASE_MAX_LEAP: u32 = 1;
const MAX_ERROR: f64 = 1.0 / 4294967296.0; // 2^-32

#[derive(GodotClass)]
#[class(base=CharacterBody3D)]
pub struct PlayerCharacter {
	base: Base<CharacterBody3D>,
	collider: Maybe<CollisionShape3D>,
	camera: Maybe<Camera3D>,
	mov: ControlState,
	air_control: f64,
	leap: u32,
	h_vel: Vector2,
	keybind: PlayerKeybinding,
}

fn third(v: Vector2) -> Vector3 {
	Vector3::new(v.x, 0.0, -v.y)
}

fn two_horizon(v: Vector3) -> Vector2 {
	Vector2::new(v.x, -v.z)
} 

fn decel2(v: Vector2, by: f64, dt: f64) -> Vector2 {
	let decel_magn = (v.length() as f64 - by * dt).max(0.0);
	let decel = if decel_magn > MAX_ERROR {
			decel_magn
	} else {
			0.0
	};
	let vel = v * decel as f32;
	vel
}

#[godot_api]
impl ICharacterBody3D for PlayerCharacter {
	fn init(base: Base<CharacterBody3D>) -> Self {
		let keybind = PlayerKeybinding {
			forward: Key::W,
			backward: Key::S,
			strafe_left: Key::A,
			strafe_right: Key::D,
			leap: Key::SPACE,
		};
		Self {
			base,
			collider: None,
			camera: None,
			mov: ControlState::Idle,
			air_control: 0.6,
			leap: 1,
			keybind,
			h_vel: Vector2::ZERO
		}
	}

	fn ready(&mut self) {
		let Some(collider) = self.base().try_get_node_as("Collider") else {
			panic!("Player initialized without a collider.")
		};
		let Some(camera) = self.base().try_get_node_as("View") else {
			panic!("Player initialized without a camera.")
		};
		self.collider = Some(collider);
		self.camera = Some(camera);
	}

	fn physics_process(&mut self, delta: f64) {
		let Some(cam) = self.camera.as_ref().map(|s| s.clone()) else {
			cold_path();
			return;
		};
		let i = Input::singleton();
		let eg = Engine::singleton();
		let mut dir = Vector2::ZERO;
		let PlayerKeybinding {
			forward,
			backward,
			strafe_left,
			strafe_right,
			leap,
		} = self.keybind;

		if i.is_key_pressed(forward) {
			dir.y += 1.0
		}
		if i.is_key_pressed(backward) {
			dir.y -= 1.0
		}
		if i.is_key_pressed(strafe_right) {
			dir.x += 1.0
		}
		if i.is_key_pressed(strafe_left) {
			dir.x -= 1.0
		}
		dir = dir.normalized();
		// these cases have some many duplicates
		// and also possibly redundant back-and-forth conversion
		match self.mov {
			ControlState::Idle => {
				if dir != Vector2::ZERO {
					let air_control = self.air_control;
					let spd = (BASE_ACCEL
						* (if self.base().is_on_floor() {
							1.0
						} else {
							air_control
						}) * delta)
						.max(BASE_MAX_SPEED);
					self.mov = ControlState::Moving { speed: spd };
					let view = cam.get_basis();
					let vel = dir * (spd * delta) as Real;
					let towards = view * third(vel);
					// let towards = view * towards;
					self.h_vel = two_horizon(towards);
				} else {
					self.h_vel = decel2(self.h_vel, BASE_DECEL, delta);
				}
			}
			ControlState::Preserving {
				tick_since_idle,
				speed,
			} => {
				let tick_span = eg.get_physics_frames() - tick_since_idle;
				if dir != Vector2::ZERO {
					let air_control = self.air_control;
					let spd = ((if tick_span <= PRESERVED_TICKS {
							speed
					} else {
							0.0
					})
						+ BASE_ACCEL
							* (if self.base().is_on_floor() {
								1.0
							} else {
								air_control
							}) * delta)
						.max(BASE_MAX_SPEED);
					self.mov = ControlState::Moving { speed: spd };
					let view = cam.get_basis();
					let vel = dir * (spd * delta) as Real;
					let towards = view * third(vel);
					// let towards = view * towards;
					self.h_vel = two_horizon(towards);
				} else if tick_span > PRESERVED_TICKS {
					self.mov = ControlState::Idle;
					self.h_vel = decel2(self.h_vel, BASE_DECEL, delta);
				} else {
					self.h_vel = decel2(self.h_vel, BASE_DECEL, delta);
				}
			}
			ControlState::Moving { speed } => {
				if dir != Vector2::ZERO {
					let air_control = self.air_control;
					let spd = (speed + BASE_ACCEL
						* (if self.base().is_on_floor() {
							1.0
						} else {
							air_control
						}) * delta)
						.max(BASE_MAX_SPEED);
					self.mov = ControlState::Moving { speed: spd };
					let view = cam.get_basis();
					let vel = dir * (spd * delta) as Real;
					let towards = view * third(vel);
					// let towards = view * towards;
					self.h_vel = two_horizon(towards);
				} else {
					self.mov = ControlState::Preserving { tick_since_idle: eg.get_physics_frames(), speed }
				}
			},
		}

		let control_h_vel = self.h_vel;
		let mut base = self.base_mut();
		let mut vor = base.get_velocity();
		vor += third(control_h_vel);
		base.set_velocity(vor);
		base.move_and_slide();
		
		// do move and slider after all calculated momentum packets?
	}
}

// self-induced movement
enum ControlState {
	Idle,
	/// Used absolute tick. This is not a tick span
	Preserving {
		tick_since_idle: u64,
		speed: f64,
	},
	Moving {
		speed: f64,
	},
}

// ideally there should also be a preserving momentum for external forces. but lets deal with that later

struct MomentumPacket {
	decel: Vector3,
	velocity: Vector3,
	cancellation: CancelKind,
}

enum CancelKind {
	// momentum cancelled by colliding with obstacles
	Any,
	// momentum cancelled by touching ground
	Grounded,
	// momentum starts dwindling when colliding with obstacles
	AnyNulling,
	// momentum starts dwindling when touching ground
	GroundNulling,
	// diminishing (towards zero) momentum
	Nulling,
}

struct PlayerKeybinding {
	forward: Key,
	backward: Key,
	strafe_left: Key,
	strafe_right: Key,
	leap: Key,
}
