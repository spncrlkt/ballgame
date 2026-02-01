//! AI decision logic for v1 brain
//!
//! This is a simplified version of the original AI logic from ballgame,
//! designed to run as a standalone client without Bevy dependencies.

use ballgame_protocol::{
    AgentInput, AgentSnapshot, BallStateKind, Basket, CharacterId,
    GameStateSnapshot, Team, Vec2,
    handshake::GameConfig,
};

/// AI state tracking
#[derive(Debug, Clone)]
pub struct AiState {
    pub current_goal: AiGoal,
    pub shot_charge_target: f32,
    pub ball_hold_time: f32,
    pub stuck_timer: f32,
    pub last_position: Option<Vec2>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            current_goal: AiGoal::ChaseBall,
            shot_charge_target: 0.0,
            ball_hold_time: 0.0,
            stuck_timer: 0.0,
            last_position: None,
        }
    }
}

/// Goals the AI can pursue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiGoal {
    /// Stand still, do nothing
    Idle,
    /// Move toward free ball and pick it up
    #[default]
    ChaseBall,
    /// Move toward basket with ball
    AttackWithBall,
    /// Charging a shot at the basket
    ChargeShot,
    /// Attempting to steal from opponent
    AttemptSteal,
    /// Defensive positioning
    DefendBasket,
}

/// AI Brain v1 - decision-making engine
pub struct BrainV1 {
    /// Which character we're controlling
    pub character: CharacterId,
    /// Our team
    pub team: Team,
    /// Game configuration
    pub config: GameConfig,
    /// Internal state
    pub state: AiState,
}

impl BrainV1 {
    pub fn new(character: CharacterId, config: GameConfig) -> Self {
        Self {
            character,
            team: character.team(),
            config,
            state: AiState::default(),
        }
    }

    /// Main decision function - returns input for this tick
    pub fn decide(&mut self, game_state: &GameStateSnapshot) -> AgentInput {
        // Find our agent in the state
        let Some(our_agent) = game_state.agents.iter().find(|a| a.character == self.character)
        else {
            return AgentInput::default();
        };

        // Update goal based on game state
        self.update_goal(game_state, our_agent);

        // Execute current goal
        self.execute_goal(game_state, our_agent)
    }

    /// Update AI goal based on current game state
    fn update_goal(&mut self, game_state: &GameStateSnapshot, our_agent: &AgentSnapshot) {
        let ball = &game_state.ball;

        // Check if we have the ball
        let we_have_ball = our_agent.holding_ball;

        // Check ball state
        let ball_holder = match ball.state {
            BallStateKind::Held { holder } => Some(holder),
            _ => None,
        };

        let opponent_has_ball = ball_holder
            .map(|h| h.team() != self.team)
            .unwrap_or(false);

        // Goal transitions
        match self.state.current_goal {
            AiGoal::Idle => {
                // Stay idle
            }
            AiGoal::ChaseBall => {
                if we_have_ball {
                    self.state.current_goal = AiGoal::AttackWithBall;
                    self.state.ball_hold_time = 0.0;
                } else if opponent_has_ball {
                    self.state.current_goal = AiGoal::AttemptSteal;
                }
            }
            AiGoal::AttackWithBall => {
                if !we_have_ball {
                    self.state.current_goal = AiGoal::ChaseBall;
                } else {
                    // Check if we're in shooting range
                    let basket_pos = self.target_basket_position(our_agent);
                    let distance = our_agent.position.distance(basket_pos);
                    if distance < 400.0 {
                        self.state.current_goal = AiGoal::ChargeShot;
                        self.state.shot_charge_target = self.calculate_charge_target(distance);
                    }
                }
            }
            AiGoal::ChargeShot => {
                if !we_have_ball {
                    self.state.current_goal = AiGoal::ChaseBall;
                }
                // Shot release is handled in execute_goal
            }
            AiGoal::AttemptSteal => {
                if we_have_ball {
                    self.state.current_goal = AiGoal::AttackWithBall;
                    self.state.ball_hold_time = 0.0;
                } else if !opponent_has_ball {
                    self.state.current_goal = AiGoal::ChaseBall;
                }
            }
            AiGoal::DefendBasket => {
                if we_have_ball {
                    self.state.current_goal = AiGoal::AttackWithBall;
                } else if !opponent_has_ball {
                    self.state.current_goal = AiGoal::ChaseBall;
                }
            }
        }
    }

    /// Execute the current goal and return input
    fn execute_goal(
        &mut self,
        game_state: &GameStateSnapshot,
        our_agent: &AgentSnapshot,
    ) -> AgentInput {
        match self.state.current_goal {
            AiGoal::Idle => AgentInput::default(),
            AiGoal::ChaseBall => self.execute_chase_ball(game_state, our_agent),
            AiGoal::AttackWithBall => self.execute_attack(game_state, our_agent),
            AiGoal::ChargeShot => self.execute_charge_shot(game_state, our_agent),
            AiGoal::AttemptSteal => self.execute_steal(game_state, our_agent),
            AiGoal::DefendBasket => self.execute_defend(game_state, our_agent),
        }
    }

    /// Chase the ball
    fn execute_chase_ball(
        &self,
        game_state: &GameStateSnapshot,
        our_agent: &AgentSnapshot,
    ) -> AgentInput {
        let ball_pos = game_state.ball.position;
        let move_x = self.move_toward(our_agent.position.x, ball_pos.x);

        let mut input = AgentInput::with_movement(move_x);

        // Try to pick up if close
        let distance = our_agent.position.distance(ball_pos);
        if distance < 60.0 {
            input = input.with_action();
        }

        // Jump if ball is above us
        if ball_pos.y > our_agent.position.y + 50.0 && our_agent.grounded {
            input = input.with_jump();
        }

        input
    }

    /// Attack toward the basket
    fn execute_attack(
        &self,
        game_state: &GameStateSnapshot,
        our_agent: &AgentSnapshot,
    ) -> AgentInput {
        let basket_pos = self.target_basket_position(our_agent);
        let move_x = self.move_toward(our_agent.position.x, basket_pos.x);

        let mut input = AgentInput::with_movement(move_x);

        // Use turbo when attacking
        input = input.with_turbo();

        input
    }

    /// Charge and release a shot
    fn execute_charge_shot(
        &mut self,
        _game_state: &GameStateSnapshot,
        our_agent: &AgentSnapshot,
    ) -> AgentInput {
        // If we're at target charge, release
        if our_agent.charge_progress >= self.state.shot_charge_target {
            self.state.current_goal = AiGoal::ChaseBall; // Will transition after shot
            return AgentInput::default().with_shoot_release();
        }

        // Keep charging
        AgentInput::default().with_shoot_held()
    }

    /// Attempt to steal from opponent
    fn execute_steal(
        &self,
        game_state: &GameStateSnapshot,
        our_agent: &AgentSnapshot,
    ) -> AgentInput {
        // Find the ball carrier (opponent)
        let ball_holder = match game_state.ball.state {
            BallStateKind::Held { holder } => {
                game_state.agents.iter().find(|a| a.character == holder)
            }
            _ => None,
        };

        let Some(carrier) = ball_holder else {
            return AgentInput::default();
        };

        let move_x = self.move_toward(our_agent.position.x, carrier.position.x);
        let mut input = AgentInput::with_movement(move_x);

        // Try to steal if close
        let distance = our_agent.position.distance(carrier.position);
        if distance < self.config.steal_range {
            input = input.with_action();
        }

        // Jump if carrier is above
        if carrier.position.y > our_agent.position.y + 30.0 && our_agent.grounded {
            input = input.with_jump();
        }

        input
    }

    /// Defend the basket
    fn execute_defend(
        &self,
        game_state: &GameStateSnapshot,
        our_agent: &AgentSnapshot,
    ) -> AgentInput {
        // Position between ball and our basket
        let our_basket_pos = self.own_basket_position(our_agent);
        let ball_pos = game_state.ball.position;

        // Stand at midpoint, biased toward basket
        let target_x = (ball_pos.x + our_basket_pos.x * 2.0) / 3.0;
        let move_x = self.move_toward(our_agent.position.x, target_x);

        AgentInput::with_movement(move_x)
    }

    // Helper methods

    fn move_toward(&self, current: f32, target: f32) -> f32 {
        let diff = target - current;
        if diff.abs() < 10.0 {
            0.0
        } else if diff > 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    fn target_basket_position(&self, our_agent: &AgentSnapshot) -> Vec2 {
        // Target the opponent's basket
        match our_agent.target_basket {
            Basket::Left => Vec2::new(-self.config.arena_width / 2.0 + 100.0, 200.0),
            Basket::Right => Vec2::new(self.config.arena_width / 2.0 - 100.0, 200.0),
        }
    }

    fn own_basket_position(&self, our_agent: &AgentSnapshot) -> Vec2 {
        // Our own basket (opposite of target)
        match our_agent.target_basket {
            Basket::Left => Vec2::new(self.config.arena_width / 2.0 - 100.0, 200.0),
            Basket::Right => Vec2::new(-self.config.arena_width / 2.0 + 100.0, 200.0),
        }
    }

    fn calculate_charge_target(&self, distance: f32) -> f32 {
        // Simple linear mapping: closer = less charge needed
        let min_charge = 0.3;
        let max_charge = 1.0;
        let max_distance = 500.0;

        let t = (distance / max_distance).clamp(0.0, 1.0);
        min_charge + t * (max_charge - min_charge)
    }
}
