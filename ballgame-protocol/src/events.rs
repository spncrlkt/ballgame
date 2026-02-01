//! Game events for network transmission

use serde::{Deserialize, Serialize};

use crate::game_state::{CharacterId, Team};

/// Game events that can be sent from server to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // === Scoring ===
    /// Goal scored
    Goal {
        scorer: CharacterId,
        score_left: u32,
        score_right: u32,
    },

    // === Ball Events ===
    /// Ball picked up by a character
    Pickup { character: CharacterId },

    /// Ball dropped/lost
    Drop { character: CharacterId },

    /// Shot started (charge began)
    ShotStart { character: CharacterId },

    /// Shot released
    ShotRelease {
        character: CharacterId,
        charge: f32,
        angle: f32,
        power: f32,
    },

    /// Pass initiated
    Pass {
        passer: CharacterId,
        target: CharacterId,
    },

    /// Pass completed
    PassCompleted {
        passer: CharacterId,
        receiver: CharacterId,
    },

    /// Pass intercepted
    PassIntercepted {
        passer: CharacterId,
        interceptor: CharacterId,
    },

    /// Pass missed
    PassMissed {
        passer: CharacterId,
        target: CharacterId,
    },

    // === Steal Events ===
    /// Steal attempted
    StealAttempt { attacker: CharacterId },

    /// Steal succeeded
    StealSuccess { attacker: CharacterId },

    /// Steal failed
    StealFail { attacker: CharacterId },

    // === Block Events ===
    /// Block activated
    BlockActivated { character: CharacterId },

    /// Block intercepted ball
    BlockIntercepted { blocker: CharacterId },

    // === Movement Events ===
    /// Character jumped
    Jump { character: CharacterId },

    /// Character landed
    Land { character: CharacterId },

    // === Match Events ===
    /// Match started
    MatchStart {
        level_id: String,
        level_name: String,
    },

    /// Match ended
    MatchEnd {
        winner: Option<Team>,
        score_left: u32,
        score_right: u32,
        duration: f32,
    },

    /// Level changed
    LevelChange { level_id: String },

    // === Countdown Events ===
    /// Countdown tick (3, 2, 1)
    CountdownTick { remaining: u32 },

    /// Countdown finished, match started
    CountdownEnd,
}

impl GameEvent {
    /// Get a short type code for logging
    pub fn type_code(&self) -> &'static str {
        match self {
            GameEvent::Goal { .. } => "GOAL",
            GameEvent::Pickup { .. } => "PICKUP",
            GameEvent::Drop { .. } => "DROP",
            GameEvent::ShotStart { .. } => "SHOT_START",
            GameEvent::ShotRelease { .. } => "SHOT_REL",
            GameEvent::Pass { .. } => "PASS",
            GameEvent::PassCompleted { .. } => "PASS_OK",
            GameEvent::PassIntercepted { .. } => "PASS_INT",
            GameEvent::PassMissed { .. } => "PASS_MISS",
            GameEvent::StealAttempt { .. } => "STEAL_TRY",
            GameEvent::StealSuccess { .. } => "STEAL_OK",
            GameEvent::StealFail { .. } => "STEAL_FAIL",
            GameEvent::BlockActivated { .. } => "BLOCK",
            GameEvent::BlockIntercepted { .. } => "BLOCK_INT",
            GameEvent::Jump { .. } => "JUMP",
            GameEvent::Land { .. } => "LAND",
            GameEvent::MatchStart { .. } => "MATCH_START",
            GameEvent::MatchEnd { .. } => "MATCH_END",
            GameEvent::LevelChange { .. } => "LEVEL_CHG",
            GameEvent::CountdownTick { .. } => "COUNTDOWN",
            GameEvent::CountdownEnd => "COUNTDOWN_END",
        }
    }

    /// Check if this event is related to scoring
    pub fn is_scoring_event(&self) -> bool {
        matches!(self, GameEvent::Goal { .. })
    }

    /// Check if this event is related to the ball
    pub fn is_ball_event(&self) -> bool {
        matches!(
            self,
            GameEvent::Pickup { .. }
                | GameEvent::Drop { .. }
                | GameEvent::ShotStart { .. }
                | GameEvent::ShotRelease { .. }
                | GameEvent::Pass { .. }
                | GameEvent::PassCompleted { .. }
                | GameEvent::PassIntercepted { .. }
                | GameEvent::PassMissed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = GameEvent::Goal {
            scorer: CharacterId::L0,
            score_left: 1,
            score_right: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: GameEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.type_code(), "GOAL");
    }

    #[test]
    fn test_event_categories() {
        assert!(GameEvent::Goal {
            scorer: CharacterId::L0,
            score_left: 1,
            score_right: 0
        }
        .is_scoring_event());
        assert!(GameEvent::Pickup {
            character: CharacterId::L0
        }
        .is_ball_event());
    }
}
