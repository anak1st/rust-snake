use std::collections::VecDeque;

use crate::config::game::{
    AI_SNAKE_COUNT, BOMB_COUNT, FOOD_COUNT, SUPER_FOOD_COUNT, SUPER_FOOD_SCORE_GAIN,
};

use super::{Direction, GameState, Position, RunState};

#[test]
/// 验证每次 tick 都会让玩家蛇头向前推进一格。
fn snake_moves_forward_on_tick() {
    let mut game = GameState::with_board_size(18, 8);
    game.start();
    let old_head = game.player().body().back().copied().unwrap();

    game.tick();

    let new_head = game.player().body().back().copied().unwrap();
    assert_eq!(new_head.x, old_head.x + 1);
    assert_eq!(new_head.y, old_head.y);
}

#[test]
/// 验证直接反向输入会被忽略，避免玩家蛇原地掉头。
fn opposite_direction_is_ignored() {
    let mut game = GameState::with_board_size(18, 8);
    game.start();
    game.set_direction(Direction::Left);

    game.tick();

    assert_eq!(game.direction(), Direction::Right);
}

#[test]
/// 验证玩家蛇撞到边界后会进入游戏结束状态。
fn wall_collision_ends_game() {
    let mut game = GameState::with_board_size(4, 4);
    game.start();

    for _ in 0..2 {
        game.tick();
    }

    assert_eq!(game.run_state(), RunState::GameOver);
}

#[test]
/// 验证新游戏默认停留在开始界面，等待玩家启动。
fn new_game_starts_in_ready_state() {
    let game = GameState::with_board_size(10, 8);

    assert_eq!(game.run_state(), RunState::Ready);
}

#[test]
/// 验证新游戏会一次生成多颗食物。
fn game_spawns_multiple_foods() {
    let game = GameState::with_board_size(12, 8);

    assert_eq!(game.foods().len(), FOOD_COUNT);
    assert_eq!(game.super_foods().len(), SUPER_FOOD_COUNT);
    assert_eq!(game.bombs().len(), BOMB_COUNT);
}

#[test]
/// 验证初始敌蛇数量正确，并且都与玩家分离。
fn enemy_snakes_start_separate_from_player() {
    let game = GameState::with_board_size(20, 10);

    assert_eq!(game.enemy_count(), AI_SNAKE_COUNT);
    assert!(
        game.enemies()
            .iter()
            .flat_map(|enemy| enemy.body().iter())
            .all(|segment| !game.player().body().contains(segment))
    );
}

#[test]
/// 验证初始敌蛇之间也不会互相重叠。
fn enemy_snakes_start_separate_from_each_other() {
    let game = GameState::with_board_size(20, 10);

    for (index, enemy) in game.enemies().iter().enumerate() {
        for other in game.enemies().iter().skip(index + 1) {
            assert!(
                enemy
                    .body()
                    .iter()
                    .all(|segment| !other.body().contains(segment))
            );
        }
    }
}

#[test]
/// 验证四条初始敌蛇优先出生在棋盘四个角落。
fn enemy_snakes_spawn_in_four_corners() {
    let game = GameState::with_board_size(20, 10);

    assert!(
        game.enemies()
            .iter()
            .any(|enemy| enemy.head() == Position { x: 0, y: 0 })
    );
    assert!(
        game.enemies()
            .iter()
            .any(|enemy| { enemy.head() == Position { x: 19, y: 0 } })
    );
    assert!(
        game.enemies()
            .iter()
            .any(|enemy| { enemy.head() == Position { x: 0, y: 9 } })
    );
    assert!(
        game.enemies()
            .iter()
            .any(|enemy| { enemy.head() == Position { x: 19, y: 9 } })
    );
}

#[test]
/// 验证玩家吃到炸弹后会立即结束游戏。
fn bomb_ends_game_for_player() {
    let mut game = GameState::with_board_size(12, 8);
    game.foods.clear();
    game.super_foods.clear();
    game.bombs = vec![Position { x: 6, y: 4 }];
    game.enemies.clear();
    game.start();

    game.tick();

    assert_eq!(game.run_state(), RunState::GameOver);
}

#[test]
/// 验证超级果实会带来额外得分，并在后续 tick 继续增长。
fn super_fruit_grants_extra_growth() {
    let mut game = GameState::with_board_size(18, 8);
    game.foods.clear();
    game.super_foods = vec![Position { x: 8, y: 4 }];
    game.bombs.clear();
    game.enemies.clear();
    game.start();

    game.tick();
    assert_eq!(game.score(), SUPER_FOOD_SCORE_GAIN);
    assert_eq!(game.player().body().len(), 4);

    game.tick();
    game.tick();

    assert_eq!(game.player().body().len(), 6);
}

#[test]
/// 验证蛇死亡后，其身体会化成普通食物留在棋盘上。
fn crashing_into_enemy_drops_legacy() {
    let mut game = GameState::with_board_size(16, 8);
    game.foods.clear();
    game.super_foods.clear();
    game.bombs.clear();
    game.player.body = VecDeque::from([
        Position { x: 1, y: 4 },
        Position { x: 2, y: 4 },
        Position { x: 3, y: 4 },
    ]);
    game.player.direction = Direction::Right;
    game.player.control = super::SnakeControl::Manual {
        pending_direction: Direction::Right,
    };
    game.enemies = vec![super::Snake::new_ai(
        VecDeque::from([
            Position { x: 6, y: 4 },
            Position { x: 5, y: 4 },
            Position { x: 4, y: 4 },
        ]),
        Direction::Left,
        super::SnakeAppearance::for_slot(0),
    )];
    game.start();

    game.tick();

    assert_eq!(game.run_state(), RunState::GameOver);
    assert_eq!(game.player().body().len(), 3);
    assert_eq!(game.enemies()[0].score(), 0);
    assert_eq!(game.foods().len(), FOOD_COUNT);
    assert!(game.legacy_foods().contains(&Position { x: 1, y: 4 }));
    assert!(game.legacy_foods().contains(&Position { x: 2, y: 4 }));
    assert!(game.legacy_foods().contains(&Position { x: 3, y: 4 }));
}

#[test]
/// 验证玩家撞进敌蛇身体时也会死亡，而不是直接穿过。
fn player_crashes_into_enemy_body() {
    let mut game = GameState::with_board_size(16, 8);
    game.foods.clear();
    game.super_foods.clear();
    game.bombs.clear();
    game.player.body = VecDeque::from([
        Position { x: 3, y: 4 },
        Position { x: 4, y: 4 },
        Position { x: 5, y: 4 },
    ]);
    game.player.direction = Direction::Right;
    game.player.control = super::SnakeControl::Manual {
        pending_direction: Direction::Right,
    };
    game.enemies = vec![super::Snake::new_ai(
        VecDeque::from([
            Position { x: 7, y: 4 },
            Position { x: 6, y: 4 },
            Position { x: 5, y: 4 },
        ]),
        Direction::Up,
        super::SnakeAppearance::for_slot(0),
    )];
    game.start();

    game.tick();

    assert_eq!(game.run_state(), RunState::GameOver);
    assert!(game.legacy_foods().contains(&Position { x: 3, y: 4 }));
    assert!(game.legacy_foods().contains(&Position { x: 4, y: 4 }));
    assert!(game.legacy_foods().contains(&Position { x: 5, y: 4 }));
}

#[test]
/// 验证尸体食物不会占用普通食物配额，补货后仍会补齐常规食物。
fn legacy_food_does_not_reduce_normal_food_refill() {
    let mut game = GameState::with_board_size(16, 8);
    game.foods.clear();
    game.super_foods.clear();
    game.bombs.clear();
    game.enemies.clear();
    game.player.body = VecDeque::from([
        Position { x: 1, y: 4 },
        Position { x: 2, y: 4 },
        Position { x: 3, y: 4 },
    ]);
    game.player.direction = Direction::Right;
    game.player.control = super::SnakeControl::Manual {
        pending_direction: Direction::Right,
    };
    game.bombs = vec![Position { x: 4, y: 4 }];
    game.start();

    game.tick();

    assert_eq!(game.run_state(), RunState::GameOver);
    assert_eq!(game.foods().len(), FOOD_COUNT);
    assert_eq!(game.legacy_foods().len(), 3);
}

#[test]
/// 验证头撞头时体型较小的一方死亡。
fn smaller_snake_loses_head_on() {
    let game = GameState::with_board_size(16, 8);
    let enemy_plans = vec![super::SnakePlan {
        next_head: Position { x: 6, y: 4 },
        consumable: None,
        growth_amount: 0,
        score_gain: 0,
        hits_bomb: false,
        navigation: super::NavigationDecision {
            direction: Direction::Left,
            random_walk_steps: 0,
            random_walk_direction: None,
        },
        crashes: false,
    }];
    let mut player_dies = false;
    let mut enemy_dies = vec![false];

    let mut game = game;
    game.player.body = VecDeque::from([
        Position { x: 3, y: 4 },
        Position { x: 4, y: 4 },
        Position { x: 5, y: 4 },
        Position { x: 5, y: 5 },
    ]);
    game.enemies = vec![super::Snake::new_ai(
        VecDeque::from([
            Position { x: 8, y: 4 },
            Position { x: 7, y: 4 },
            Position { x: 6, y: 4 },
        ]),
        Direction::Left,
        super::SnakeAppearance::for_slot(0),
    )];

    game.resolve_player_enemy_head_on(
        Position { x: 6, y: 4 },
        super::TileEffect {
            consumable: None,
            growth_amount: 0,
            score_gain: 0,
            hits_bomb: false,
        },
        &enemy_plans,
        &mut player_dies,
        &mut enemy_dies,
    );

    assert!(!player_dies);
    assert_eq!(enemy_dies, vec![true]);
}
