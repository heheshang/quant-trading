//! Signal Fusion Logic for AI Quant Module
//!
//! Combines AI model signals with rule-based signals using weighted fusion.
//!
//! Fusion rules (from Contract-Review-AI-Quant.md):
//! | AI confidence  | Rule confidence | AI weight | Rule weight |
//! |----------------|-----------------|-----------|-------------|
//! | > 0.8          | any             | 0.7       | 0.3         |
//! | 0.6 – 0.8      | > 0.7           | 0.6       | 0.4         |
//! | 0.6 – 0.8      | 0.5 – 0.7       | 0.5       | 0.5         |
//! | < 0.6          | any             | 0.4       | 0.6         |
//!
//! Conflict rule: if AI and rule directions disagree AND AI confidence < 0.4,
//! the rule direction is returned directly (weights are ignored).

/// Direction of a trading signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Buy / Long
    Long,
    /// Sell / Short
    Short,
    /// No position / Neutral
    Neutral,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Long => write!(f, "long"),
            Direction::Short => write!(f, "short"),
            Direction::Neutral => write!(f, "neutral"),
        }
    }
}

/// AI model signal with its confidence score.
#[derive(Debug, Clone, Copy)]
pub struct AiSignal {
    /// Trading direction recommended by the AI model.
    pub direction: Direction,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f64,
}

/// Rule-based signal with its confidence score.
#[derive(Debug, Clone, Copy)]
pub struct RuleSignal {
    /// Trading direction recommended by the rule.
    pub direction: Direction,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f64,
}

/// Result of fusing an AI signal with a rule signal.
#[derive(Debug, Clone, Copy)]
pub struct FusedSignal {
    /// Final trading direction after fusion.
    pub direction: Direction,
    /// Final confidence score in [0.0, 1.0].
    pub confidence: f64,
    /// Fusion weights used (ai_weight, rule_weight).
    pub weights: (f64, f64),
}

/// Fusion weight variant selected based on AI and rule confidence levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionWeight {
    /// AI > 0.8, any rule  →  AI 0.7, Rule 0.3
    AiDominant,
    /// AI 0.6-0.8, rule > 0.7  →  AI 0.6, Rule 0.4
    AiStrongRuleVeryStrong,
    /// AI 0.6-0.8, rule 0.5-0.7  →  AI 0.5, Rule 0.5
    Balanced,
    /// AI < 0.6, any rule  →  AI 0.4, Rule 0.6
    RuleDominant,
}

impl FusionWeight {
    /// Returns the (ai_weight, rule_weight) tuple for this fusion variant.
    pub fn weights(self) -> (f64, f64) {
        match self {
            FusionWeight::AiDominant => (0.7, 0.3),
            FusionWeight::AiStrongRuleVeryStrong => (0.6, 0.4),
            FusionWeight::Balanced => (0.5, 0.5),
            FusionWeight::RuleDominant => (0.4, 0.6),
        }
    }
}

/// Determines the fusion weight variant from AI and rule confidence scores.
///
/// # Arguments
/// * `ai_confidence` - AI model confidence in [0.0, 1.0]
/// * `rule_confidence` - Rule signal confidence in [0.0, 1.0]
pub fn determine_fusion_weight(ai_confidence: f64, rule_confidence: f64) -> FusionWeight {
    if ai_confidence > 0.8 {
        FusionWeight::AiDominant
    } else if ai_confidence >= 0.6 {
        if rule_confidence > 0.7 {
            FusionWeight::AiStrongRuleVeryStrong
        } else {
            FusionWeight::Balanced
        }
    } else {
        FusionWeight::RuleDominant
    }
}

/// Checks whether the AI signal confidence is below the conflict threshold.
fn is_ai_weak(ai_confidence: f64) -> bool {
    ai_confidence < 0.4
}

/// Checks whether two directions conflict (are opposite non-neutral directions).
fn directions_conflict(ai_dir: Direction, rule_dir: Direction) -> bool {
    matches!(
        (ai_dir, rule_dir),
        (Direction::Long, Direction::Short) | (Direction::Short, Direction::Long)
    )
}

/// Computes the fused confidence from two confidence scores and their weights.
fn compute_fused_confidence(
    ai_confidence: f64,
    rule_confidence: f64,
    ai_weight: f64,
    rule_weight: f64,
) -> f64 {
    ai_confidence * ai_weight + rule_confidence * rule_weight
}

/// Fuses an AI signal with a rule-based signal using weighted fusion.
///
/// The function applies the following logic in order:
/// 1. **Conflict rule**: if directions disagree AND AI confidence < 0.4,
///    return the rule direction with rule confidence (weights = (0.0, 1.0)).
/// 2. **Fusion weights**: select weights based on the 4-level table above.
/// 3. **Direction**: if directions agree (or AI is strong enough), use the
///    majority-weighted direction (always Long when agreeing since both
///    signals point the same way).
/// 4. **Confidence**: weighted average of both confidences.
///
/// # Arguments
/// * `ai_signal` - Signal from the AI model
/// * `rule_signal` - Signal from the rule-based system
///
/// # Returns
/// A `FusedSignal` with the resolved direction, confidence, and applied weights.
pub fn fuse_signals(ai_signal: AiSignal, rule_signal: RuleSignal) -> FusedSignal {
    // --- Conflict rule -------------------------------------------------------
    if directions_conflict(ai_signal.direction, rule_signal.direction)
        && is_ai_weak(ai_signal.confidence)
    {
        return FusedSignal {
            direction: rule_signal.direction,
            confidence: rule_signal.confidence,
            weights: (0.0, 1.0),
        };
    }

    // --- Determine fusion weights ---------------------------------------------
    let fw = determine_fusion_weight(ai_signal.confidence, rule_signal.confidence);
    let (ai_weight, rule_weight) = fw.weights();

    // --- Resolve direction (they agree or AI is strong enough) --------------
    let direction = if ai_signal.direction == rule_signal.direction {
        ai_signal.direction
    } else {
        // Directions disagree but AI confidence >= 0.4 — use weighted fusion
        // which will produce a moderate confidence; default to AI direction
        // as the more "decisive" signal
        ai_signal.direction
    };

    // --- Compute confidence ---------------------------------------------------
    let confidence = compute_fused_confidence(
        ai_signal.confidence,
        rule_signal.confidence,
        ai_weight,
        rule_weight,
    );

    FusedSignal {
        direction,
        confidence,
        weights: (ai_weight, rule_weight),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Fusion weight cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_fusion_ai_dominant() {
        // AI > 0.8, any rule → AI 0.7, Rule 0.3
        let ai = AiSignal {
            direction: Direction::Long,
            confidence: 0.85,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.6,
        };
        let result = fuse_signals(ai, rule);
        assert_eq!(result.direction, Direction::Long);
        assert!((result.weights.0 - 0.7).abs() < 1e-9);
        assert!((result.weights.1 - 0.3).abs() < 1e-9);
        let expected_conf = 0.85 * 0.7 + 0.6 * 0.3;
        assert!((result.confidence - expected_conf).abs() < 1e-9);
    }

    #[test]
    fn test_fusion_ai_strong_rule_very_strong() {
        // AI 0.6-0.8, rule > 0.7 → AI 0.6, Rule 0.4
        let ai = AiSignal {
            direction: Direction::Long,
            confidence: 0.75,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.8,
        };
        let result = fuse_signals(ai, rule);
        assert_eq!(result.direction, Direction::Long);
        assert!((result.weights.0 - 0.6).abs() < 1e-9);
        assert!((result.weights.1 - 0.4).abs() < 1e-9);
        let expected_conf = 0.75 * 0.6 + 0.8 * 0.4;
        assert!((result.confidence - expected_conf).abs() < 1e-9);
    }

    #[test]
    fn test_fusion_balanced() {
        // AI 0.6-0.8, rule 0.5-0.7 → AI 0.5, Rule 0.5
        let ai = AiSignal {
            direction: Direction::Short,
            confidence: 0.65,
        };
        let rule = RuleSignal {
            direction: Direction::Short,
            confidence: 0.6,
        };
        let result = fuse_signals(ai, rule);
        assert_eq!(result.direction, Direction::Short);
        assert!((result.weights.0 - 0.5).abs() < 1e-9);
        assert!((result.weights.1 - 0.5).abs() < 1e-9);
        let expected_conf = 0.65 * 0.5 + 0.6 * 0.5;
        assert!((result.confidence - expected_conf).abs() < 1e-9);
    }

    #[test]
    fn test_fusion_rule_dominant() {
        // AI < 0.6, any rule → AI 0.4, Rule 0.6
        let ai = AiSignal {
            direction: Direction::Long,
            confidence: 0.55,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.9,
        };
        let result = fuse_signals(ai, rule);
        assert_eq!(result.direction, Direction::Long);
        assert!((result.weights.0 - 0.4).abs() < 1e-9);
        assert!((result.weights.1 - 0.6).abs() < 1e-9);
        let expected_conf = 0.55 * 0.4 + 0.9 * 0.6;
        assert!((result.confidence - expected_conf).abs() < 1e-9);
    }

    // -------------------------------------------------------------------------
    // Conflict cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_conflict_rule_wins_when_ai_weak() {
        // Directions disagree AND AI confidence < 0.4 → rule wins
        let ai = AiSignal {
            direction: Direction::Short,
            confidence: 0.35,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.7,
        };
        let result = fuse_signals(ai, rule);
        assert_eq!(result.direction, Direction::Long); // rule direction
        assert!((result.confidence - 0.7).abs() < 1e-9); // rule confidence
        assert!((result.weights.0 - 0.0).abs() < 1e-9);
        assert!((result.weights.1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_conflict_ai_wins_when_ai_strong_enough() {
        // Directions disagree but AI confidence >= 0.4 → fusion applies
        let ai = AiSignal {
            direction: Direction::Short,
            confidence: 0.6,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.7,
        };
        let result = fuse_signals(ai, rule);
        // AI=0.6, rule=0.7 → rule > 0.7 is false → Balanced (0.5, 0.5)
        assert!((result.weights.0 - 0.5).abs() < 1e-9);
        assert!((result.weights.1 - 0.5).abs() < 1e-9);
        // Direction falls back to AI direction when conflicting
        assert_eq!(result.direction, Direction::Short);
    }

    // -------------------------------------------------------------------------
    // Edge / boundary cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_ai_confidence_exactly_0_8() {
        // Exactly 0.8 should fall into the 0.6-0.8 bucket (not > 0.8)
        let ai = AiSignal {
            direction: Direction::Long,
            confidence: 0.8,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.6,
        };
        let result = fuse_signals(ai, rule);
        // rule confidence 0.6 is in 0.5-0.7 range → Balanced
        assert!((result.weights.0 - 0.5).abs() < 1e-9);
        assert!((result.weights.1 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_ai_confidence_exactly_0_6() {
        let ai = AiSignal {
            direction: Direction::Long,
            confidence: 0.6,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.75,
        };
        let result = fuse_signals(ai, rule);
        // rule > 0.7 → AiStrongRuleVeryStrong (0.6, 0.4)
        assert!((result.weights.0 - 0.6).abs() < 1e-9);
        assert!((result.weights.1 - 0.4).abs() < 1e-9);
    }

    #[test]
    fn test_rule_confidence_exactly_0_7() {
        let ai = AiSignal {
            direction: Direction::Long,
            confidence: 0.7,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.7,
        };
        let result = fuse_signals(ai, rule);
        // rule exactly 0.7 → falls into Balanced (0.5-0.7), NOT > 0.7
        assert!((result.weights.0 - 0.5).abs() < 1e-9);
        assert!((result.weights.1 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_neutral_directions_no_conflict() {
        let ai = AiSignal {
            direction: Direction::Neutral,
            confidence: 0.9,
        };
        let rule = RuleSignal {
            direction: Direction::Long,
            confidence: 0.5,
        };
        let result = fuse_signals(ai, rule);
        // Neutral vs Long do not conflict → AiDominant (0.7, 0.3)
        assert!((result.weights.0 - 0.7).abs() < 1e-9);
        assert!((result.weights.1 - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_both_neutral() {
        let ai = AiSignal {
            direction: Direction::Neutral,
            confidence: 0.5,
        };
        let rule = RuleSignal {
            direction: Direction::Neutral,
            confidence: 0.4,
        };
        let result = fuse_signals(ai, rule);
        assert_eq!(result.direction, Direction::Neutral);
        // AI 0.5 < 0.6 → RuleDominant (0.4, 0.6)
        assert!((result.weights.0 - 0.4).abs() < 1e-9);
        assert!((result.weights.1 - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_determine_fusion_weight_ai_dominant() {
        assert_eq!(determine_fusion_weight(0.9, 0.3), FusionWeight::AiDominant);
        assert_eq!(determine_fusion_weight(0.9, 0.9), FusionWeight::AiDominant);
    }

    #[test]
    fn test_determine_fusion_weight_ai_strong_rule_very_strong() {
        assert_eq!(
            determine_fusion_weight(0.7, 0.8),
            FusionWeight::AiStrongRuleVeryStrong
        );
        assert_eq!(
            determine_fusion_weight(0.65, 0.75),
            FusionWeight::AiStrongRuleVeryStrong
        );
    }

    #[test]
    fn test_determine_fusion_weight_rule_dominant() {
        assert_eq!(
            determine_fusion_weight(0.5, 0.9),
            FusionWeight::RuleDominant
        );
        assert_eq!(
            determine_fusion_weight(0.3, 0.3),
            FusionWeight::RuleDominant
        );
        assert_eq!(
            determine_fusion_weight(0.59, 0.99),
            FusionWeight::RuleDominant
        );
    }

    #[test]
    fn test_fusion_weight_weights() {
        assert_eq!(FusionWeight::AiDominant.weights(), (0.7, 0.3));
        assert_eq!(FusionWeight::AiStrongRuleVeryStrong.weights(), (0.6, 0.4));
        assert_eq!(FusionWeight::Balanced.weights(), (0.5, 0.5));
        assert_eq!(FusionWeight::RuleDominant.weights(), (0.4, 0.6));
    }
}
