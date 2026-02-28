use rand::Rng;

pub const ARENA_WIDTH: f32 = 800.0;
pub const ARENA_HEIGHT: f32 = 600.0;
pub const SHIP_ROTATION_SPEED: f32 = 5.0;
pub const SHIP_THRUST: f32 = 200.0;
pub const SHIP_DRAG: f32 = 0.98;
pub const PROJECTILE_SPEED: f32 = 400.0;
pub const PROJECTILE_LIFETIME: f32 = 2.0;
pub const FIRE_COOLDOWN: f32 = 0.25;
pub const MATCH_DURATION: f32 = 30.0;
pub const SHIP_RADIUS: f32 = 12.0;
pub const PROJECTILE_RADIUS: f32 = 2.0;
pub const MAX_PROJECTILES_PER_SHIP: usize = 5;
pub const MAX_SHIP_SPEED: f32 = 300.0;

#[derive(Clone, Debug)]
pub struct Ship {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub rotation: f32,
    pub alive: bool,
    pub fire_cooldown: f32,
    pub shots_fired: usize,
    pub hits_scored: usize,
}

#[derive(Clone, Debug)]
pub struct Projectile {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub lifetime: f32,
    pub owner: usize,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub ships: [Ship; 2],
    pub projectiles: Vec<Projectile>,
    pub time: f32,
    pub match_over: bool,
    pub winner: Option<usize>,
}

impl Ship {
    pub fn new(x: f32, y: f32, rotation: f32) -> Self {
        Ship {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            rotation,
            alive: true,
            fire_cooldown: 0.0,
            shots_fired: 0,
            hits_scored: 0,
        }
    }
}

impl GameState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        GameState {
            ships: [
                Ship::new(200.0, 300.0, 0.0),
                Ship::new(600.0, 300.0, std::f32::consts::PI),
            ],
            projectiles: Vec::new(),
            time: 0.0,
            match_over: false,
            winner: None,
        }
    }

    pub fn new_random(rng: &mut impl Rng) -> Self {
        let y1 = rng.gen_range(150.0..450.0);
        let y2 = rng.gen_range(150.0..450.0);
        GameState {
            ships: [
                Ship::new(200.0, y1, rng.gen_range(-0.5..0.5)),
                Ship::new(600.0, y2, std::f32::consts::PI + rng.gen_range(-0.5..0.5)),
            ],
            projectiles: Vec::new(),
            time: 0.0,
            match_over: false,
            winner: None,
        }
    }

    pub fn update(&mut self, dt: f32, actions: &[[f32; 4]; 2]) {
        if self.match_over {
            self.time += dt;
            return;
        }

        self.time += dt;

        // Update ships
        for i in 0..2 {
            if !self.ships[i].alive {
                continue;
            }

            let a = &actions[i];
            let thrust = a[0].clamp(0.0, 1.0);
            let turn_left = a[1].clamp(0.0, 1.0);
            let turn_right = a[2].clamp(0.0, 1.0);
            let fire = a[3];

            // Rotation
            self.ships[i].rotation += (turn_right - turn_left) * SHIP_ROTATION_SPEED * dt;

            // Thrust
            let cos = self.ships[i].rotation.cos();
            let sin = self.ships[i].rotation.sin();
            self.ships[i].vx += cos * thrust * SHIP_THRUST * dt;
            self.ships[i].vy += sin * thrust * SHIP_THRUST * dt;

            // Drag
            let drag = SHIP_DRAG.powf(dt * 60.0);
            self.ships[i].vx *= drag;
            self.ships[i].vy *= drag;

            // Speed cap
            let speed = (self.ships[i].vx * self.ships[i].vx
                + self.ships[i].vy * self.ships[i].vy)
                .sqrt();
            if speed > MAX_SHIP_SPEED {
                let scale = MAX_SHIP_SPEED / speed;
                self.ships[i].vx *= scale;
                self.ships[i].vy *= scale;
            }

            // Position
            self.ships[i].x += self.ships[i].vx * dt;
            self.ships[i].y += self.ships[i].vy * dt;

            // Toroidal wrapping
            self.ships[i].x = wrap(self.ships[i].x, ARENA_WIDTH);
            self.ships[i].y = wrap(self.ships[i].y, ARENA_HEIGHT);

            // Fire cooldown
            self.ships[i].fire_cooldown = (self.ships[i].fire_cooldown - dt).max(0.0);

            // Fire
            if fire > 0.5 && self.ships[i].fire_cooldown <= 0.0 {
                let own_projectiles = self.projectiles.iter().filter(|p| p.owner == i).count();
                if own_projectiles < MAX_PROJECTILES_PER_SHIP {
                    self.projectiles.push(Projectile {
                        x: self.ships[i].x + cos * SHIP_RADIUS,
                        y: self.ships[i].y + sin * SHIP_RADIUS,
                        vx: cos * PROJECTILE_SPEED + self.ships[i].vx * 0.3,
                        vy: sin * PROJECTILE_SPEED + self.ships[i].vy * 0.3,
                        lifetime: PROJECTILE_LIFETIME,
                        owner: i,
                    });
                    self.ships[i].fire_cooldown = FIRE_COOLDOWN;
                    self.ships[i].shots_fired += 1;
                }
            }
        }

        // Ship-to-ship collision (elastic bounce)
        if self.ships[0].alive && self.ships[1].alive {
            let dx = toroidal_diff(self.ships[0].x, self.ships[1].x, ARENA_WIDTH);
            let dy = toroidal_diff(self.ships[0].y, self.ships[1].y, ARENA_HEIGHT);
            let dist_sq = dx * dx + dy * dy;
            let min_dist = SHIP_RADIUS * 2.0;
            if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                let dist = dist_sq.sqrt();
                let nx = dx / dist;
                let ny = dy / dist;

                // Separate ships so they don't overlap
                let overlap = min_dist - dist;
                self.ships[0].x += nx * overlap * 0.5;
                self.ships[0].y += ny * overlap * 0.5;
                self.ships[1].x -= nx * overlap * 0.5;
                self.ships[1].y -= ny * overlap * 0.5;

                // Wrap positions after separation
                self.ships[0].x = wrap(self.ships[0].x, ARENA_WIDTH);
                self.ships[0].y = wrap(self.ships[0].y, ARENA_HEIGHT);
                self.ships[1].x = wrap(self.ships[1].x, ARENA_WIDTH);
                self.ships[1].y = wrap(self.ships[1].y, ARENA_HEIGHT);

                // Elastic velocity exchange along collision normal
                let rel_vn = (self.ships[0].vx - self.ships[1].vx) * nx
                    + (self.ships[0].vy - self.ships[1].vy) * ny;
                if rel_vn < 0.0 {
                    // Ships are approaching
                    self.ships[0].vx -= rel_vn * nx;
                    self.ships[0].vy -= rel_vn * ny;
                    self.ships[1].vx += rel_vn * nx;
                    self.ships[1].vy += rel_vn * ny;
                }
            }
        }

        // Update projectiles
        for p in &mut self.projectiles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.x = wrap(p.x, ARENA_WIDTH);
            p.y = wrap(p.y, ARENA_HEIGHT);
            p.lifetime -= dt;
        }
        self.projectiles.retain(|p| p.lifetime > 0.0);

        // Collision detection
        let mut dead_projectiles = Vec::new();
        for (pi, p) in self.projectiles.iter().enumerate() {
            let target = 1 - p.owner;
            if !self.ships[target].alive {
                continue;
            }
            let dx = toroidal_diff(p.x, self.ships[target].x, ARENA_WIDTH);
            let dy = toroidal_diff(p.y, self.ships[target].y, ARENA_HEIGHT);
            let dist_sq = dx * dx + dy * dy;
            let hit_radius = SHIP_RADIUS + PROJECTILE_RADIUS;
            if dist_sq < hit_radius * hit_radius {
                self.ships[target].alive = false;
                self.ships[p.owner].hits_scored += 1;
                dead_projectiles.push(pi);
            }
        }
        // Remove hit projectiles in reverse order
        dead_projectiles.sort_unstable();
        for &pi in dead_projectiles.iter().rev() {
            self.projectiles.remove(pi);
        }

        // Check match end
        let alive_count = self.ships.iter().filter(|s| s.alive).count();
        if alive_count <= 1 || self.time >= MATCH_DURATION {
            self.match_over = true;
            if self.ships[0].alive && !self.ships[1].alive {
                self.winner = Some(0);
            } else if self.ships[1].alive && !self.ships[0].alive {
                self.winner = Some(1);
            }
        }
    }
}

pub fn wrap(val: f32, max: f32) -> f32 {
    ((val % max) + max) % max
}

pub fn toroidal_diff(a: f32, b: f32, max: f32) -> f32 {
    let d = a - b;
    if d > max / 2.0 {
        d - max
    } else if d < -max / 2.0 {
        d + max
    } else {
        d
    }
}
