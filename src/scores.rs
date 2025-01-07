use crate::shapes::Shape;

pub struct Scores;

impl Scores {
  pub fn get(shape: Shape) -> i32 {
    match shape {
      Shape::FIVE => 1_000_000_0,
      Shape::BLOCK_FIVE => 1_000_000_0,

      Shape::OPEN_FOUR => 500_000,
      Shape::SEMIOPEN_FOUR => 200_000,
      Shape::CLOSED_FOUR => 50_000,
      Shape::FOUR_FOUR => 700_000,
      Shape::FOUR_THREE => 600_000,

      Shape::OPEN_THREE => 50_000,
      Shape::SEMIOPEN_THREE => 15_000,
      Shape::CLOSED_THREE => 2_000,
      Shape::THREE_THREE => 300_000,
      Shape::SPLIT_THREE => 40_000,

      Shape::OPEN_TWO => 1_000,
      Shape::SEMIOPEN_TWO => 300,
      Shape::CLOSED_TWO => 50,
      Shape::TWO_TWO => 5_000,

      Shape::OPEN_ONE => 10,
      Shape::SEMIOPEN_ONE => 3,
      Shape::CLOSED_ONE => 1,

      Shape::DOUBLE_THREAT => 500_000,
      Shape::CROSS_THREAT => 100_000,

      Shape::NONE => 0,
    }
  }
}
