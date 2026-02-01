//! Scoring module - score tracking and check_scoring system

use bevy::prelude::*;

use crate::ai::{AiGoal, AiNavState, AiState, InputState};
use crate::ball::{Ball, BallState, CurrentPalette, Velocity};
use crate::constants::*;
use crate::events::{CharacterId, EventBus, GameEvent, TeamId};
use crate::palettes::PaletteDatabase;
use crate::player::{Character, HoldingBall, Player, Team};
use crate::ui::ScoreFlash;
use crate::world::Basket;

/// Record of a single goal for attribution tracking
#[derive(Debug, Clone)]
pub struct GoalRecord {
    /// Which character scored (if known)
    pub scorer: Option<CharacterId>,
    /// Which team scored
    pub team: TeamId,
    /// Points awarded (1 for throw, 2 for carry-in)
    pub points: u32,
    /// Match time when goal was scored (milliseconds)
    pub time_ms: u32,
}

/// Score resource tracking left/right team scores with attribution
#[derive(Resource, Default)]
pub struct Score {
    pub left: u32,  // Left team's total score
    pub right: u32, // Right team's total score
    /// Per-character goal attribution
    pub attribution: Vec<GoalRecord>,
}

impl Score {
    /// Get goals scored by a specific character
    pub fn goals_by_character(&self, character: CharacterId) -> u32 {
        self.attribution
            .iter()
            .filter(|g| g.scorer == Some(character))
            .map(|g| g.points)
            .sum()
    }

    /// Get goals scored by a team
    pub fn goals_by_team(&self, team: TeamId) -> u32 {
        match team {
            TeamId::Left => self.left,
            TeamId::Right => self.right,
        }
    }

    /// Reset scores to 0-0
    pub fn reset(&mut self) {
        self.left = 0;
        self.right = 0;
        self.attribution.clear();
    }
}

/// Current level (stores level ID)
#[derive(Resource)]
pub struct CurrentLevel(pub String);

impl Default for CurrentLevel {
    fn default() -> Self {
        // Default to empty string; should be set to first level's ID on initialization
        Self(String::new())
    }
}

/// Game pause state - toggled by Start button single press
#[derive(Resource, Default)]
pub struct GamePaused(pub bool);

/// Flag to request level restart - set by double-click Start, consumed by respawn_player
#[derive(Resource, Default)]
pub struct RestartRequested(pub bool);

/// Run condition: returns true when game is not paused
pub fn not_paused(game_paused: Res<GamePaused>) -> bool {
    !game_paused.0
}

/// Check if ball entered a basket and award points.
/// Emits Goal events to EventBus for auditability.
pub fn check_scoring(
    mut commands: Commands,
    mut score: ResMut<Score>,
    current_palette: Res<CurrentPalette>,
    palette_db: Res<PaletteDatabase>,
    mut event_bus: ResMut<EventBus>,
    time: Res<Time>,
    mut ball_query: Query<(&mut Transform, &mut Velocity, &mut BallState, &Sprite), With<Ball>>,
    basket_query: Query<(Entity, &Transform, &Basket, &Sprite), Without<Ball>>,
    player_query: Query<(Entity, &Sprite, &Team, Option<&Character>), With<Player>>,
    mut ai_query: Query<(&mut AiState, &mut AiNavState, &mut InputState), With<Player>>,
) {
    let palette = palette_db
        .get(current_palette.0)
        .expect("Palette index out of bounds");
    for (mut ball_transform, mut ball_velocity, mut ball_state, _ball_sprite) in &mut ball_query {
        let ball_pos = ball_transform.translation.truncate();
        let is_held = matches!(*ball_state, BallState::Held(_));

        for (basket_entity, basket_transform, basket, basket_sprite) in &basket_query {
            let basket_size = basket_sprite.custom_size.unwrap_or(BASKET_SIZE);
            let basket_pos = basket_transform.translation.truncate();
            let basket_half = basket_size / 2.0;

            // Check if ball center is inside basket
            let in_basket = ball_pos.x > basket_pos.x - basket_half.x
                && ball_pos.x < basket_pos.x + basket_half.x
                && ball_pos.y > basket_pos.y - basket_half.y
                && ball_pos.y < basket_pos.y + basket_half.y;

            if in_basket {
                // Determine points: 2 for carry-in, 1 for throw
                let points = if is_held { 2 } else { 1 };

                // Determine which team scored and the scoring character
                let (scoring_team_id, scorer_entity) = match basket {
                    Basket::Left => {
                        score.right += points; // Right team scores in left basket
                        (TeamId::Right, get_scorer_entity(&ball_state))
                    }
                    Basket::Right => {
                        score.left += points; // Left team scores in right basket
                        (TeamId::Left, get_scorer_entity(&ball_state))
                    }
                };

                // Get the CharacterId of the scorer (if we can determine it)
                let scorer_character: Option<CharacterId> = scorer_entity.and_then(|entity| {
                    player_query
                        .get(entity)
                        .ok()
                        .and_then(|(_, _, _, char)| char.map(|c| c.0))
                });

                // Record goal attribution
                let time_ms = (time.elapsed_secs() * 1000.0) as u32;
                score.attribution.push(GoalRecord {
                    scorer: scorer_character,
                    team: scoring_team_id,
                    points,
                    time_ms,
                });

                // Emit Goal event with CharacterId
                // Use scorer_character if known, otherwise derive from team
                let character = scorer_character.unwrap_or_else(|| match scoring_team_id {
                    TeamId::Left => CharacterId::L0,
                    TeamId::Right => CharacterId::R0,
                });
                event_bus.emit(GameEvent::Goal {
                    character,
                    score_left: score.left,
                    score_right: score.right,
                });

                // Basket color based on its side (from current palette)
                let basket_original_color = match basket {
                    Basket::Left => palette.left,
                    Basket::Right => palette.right,
                };

                // Flash the basket (gold/yellow for carry-in, white for throw)
                let flash_color = if is_held {
                    Color::srgb(1.0, 0.85, 0.0) // Gold for 2-point carry
                } else {
                    Color::srgb(1.0, 1.0, 1.0) // White for 1-point throw
                };
                commands.entity(basket_entity).insert(ScoreFlash {
                    timer: 0.6,
                    flash_color,
                    original_color: basket_original_color,
                });

                // If held, also flash the player who scored
                if let BallState::Held(holder) = *ball_state {
                    if let Ok((player_entity, _player_sprite, team, _)) = player_query.get(holder) {
                        // Player color based on team (from current palette)
                        let player_original_color = match team {
                            Team::Left => palette.left,
                            Team::Right => palette.right,
                        };
                        commands.entity(player_entity).insert(ScoreFlash {
                            timer: 0.6,
                            flash_color,
                            original_color: player_original_color,
                        });
                        // Remove HoldingBall from the player
                        commands.entity(player_entity).remove::<HoldingBall>();
                    }
                }

                // Reset ball to center
                ball_transform.translation = BALL_SPAWN;
                ball_velocity.0 = Vec2::ZERO;
                *ball_state = BallState::Free;

                // Reset ALL AI state for all players - complete reset to chase ball
                for (mut ai_state, mut nav_state, mut input_state) in &mut ai_query {
                    // Reset AI decision state
                    ai_state.current_goal = AiGoal::ChaseBall;
                    ai_state.shot_charge_target = 0.0;
                    ai_state.jump_shot_active = false;
                    ai_state.jump_shot_timer = 0.0;
                    ai_state.nav_target = None;
                    ai_state.last_position = None;
                    ai_state.stuck_timer = 0.0;
                    // Note: profile_index is NOT reset - it's the AI's personality

                    // Reset navigation state
                    nav_state.clear();

                    // Reset input state to prevent stale inputs
                    *input_state = InputState::default();
                }

                let scorer_str = scorer_character
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string());
                info!(
                    "SCORE {}pts by {}! Left: {} Right: {}",
                    points, scorer_str, score.left, score.right
                );
            }
        }
    }
}

/// Helper to get the entity that caused the score (holder or shooter/passer)
fn get_scorer_entity(ball_state: &BallState) -> Option<Entity> {
    match ball_state {
        BallState::Held(holder) => Some(*holder),
        BallState::InFlight { shooter, .. } => Some(*shooter),
        BallState::PassInFlight { passer, .. } => Some(*passer),
        BallState::Free => None,
    }
}
