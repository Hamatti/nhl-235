/**
 * NHL-235 is a command line tool for showing NHL results from previous day or current
 * in a format that's mimicing YLE's Tekstitv aesthetics
 *
 * 235 in the name refers to tekstitv page 235 which has for decades shown NHL results
 * and is a cultural piece for hockey fans in Finland
 *
 * Uses https://github.com/peruukki/nhl-score-api API for score info
 */

#[macro_use]
extern crate colour;
use reqwest::Error;
use serde_json;
use structopt::StructOpt;

use itertools::{EitherOrBoth::*, Itertools};

#[derive(Debug)]
struct Goal {
    scorer: String,
    finn_assist: String,
    minute: u64,
    finn: bool,
    special: bool,
    team: String,
}

#[derive(Debug)]
struct Game {
    home: String,
    away: String,
    score: String,
    goals: Vec<Goal>,
    status: String,
    special: String,
}

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(long)]
    version: bool,
}

fn main() {
    let args = Cli::from_args();
    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
    } else {
        match api() {
            Ok(_) => (),
            Err(err) => println!("{:?}", err),
        };
    }
}

fn translate_team_name(abbr: &str) -> String {
    let str_form = match abbr {
        "BOS" => "Boston",
        "BUF" => "Buffalo",
        "NJD" => "New Jersey",
        "NYI" => "NY Islanders",
        "NYR" => "NY Rangers",
        "PHI" => "Philadelphia",
        "PIT" => "Pittsburgh",
        "WSH" => "Washington",
        "CAR" => "Carolina",
        "CHI" => "Chicago",
        "CBJ" => "Columbus",
        "DAL" => "Dallas",
        "DET" => "Detroit",
        "FLA" => "Florida",
        "NSH" => "Nashville",
        "TBL" => "Tampa Bay",
        "ANA" => "Anaheim",
        "ARI" => "Arizona",
        "COL" => "Colorado",
        "LAK" => "Los Angeles",
        "MIN" => "Minnesota",
        "SJS" => "San Jose",
        "STL" => "St. Louis",
        "VGK" => "Vegas",
        "CGY" => "Calgary",
        "EDM" => "Edmonton",
        "MTL" => "Montreal",
        "OTT" => "Ottawa",
        "TOR" => "Toronto",
        "VAN" => "Vancouver",
        "WPG" => "Winnipeg",
        _ => "[unknown]",
    };

    String::from(str_form)
}

#[tokio::main]
async fn api() -> Result<(), Error> {
    let request_url = String::from("https://nhl-score-api.herokuapp.com/api/scores/latest");
    let response = reqwest::get(&request_url).await?;
    let scores: serde_json::Value = response.json().await?;

    let games = scores["games"].as_array().unwrap();

    let itergames = games.iter();

    let _results = itergames
        .map(|game| parse_game(&game))
        .collect::<Vec<Game>>();

    Ok(())
}

fn format_minute(min: u64, period: &str) -> u64 {
    match period {
        "1" => min,
        "2" => 20 + min,
        "3" => 40 + min,
        "OT" => 60 + min,
        _ => min,
    }
}

fn is_special(goal: &serde_json::Value) -> bool {
    let period = goal["period"].as_str().unwrap();
    let is_ot = period == "OT";
    let is_so = period == "SO";
    is_ot || is_so
}

fn parse_game(game_json: &serde_json::Value) -> Game {
    let home_team = &game_json["teams"]["home"]["abbreviation"].as_str().unwrap();
    let away_team = &game_json["teams"]["away"]["abbreviation"].as_str().unwrap();
    let home_score = &game_json["scores"][home_team];
    let away_score = &game_json["scores"][away_team];

    let empty: Vec<serde_json::Value> = Vec::new();

    let all_goals = game_json["goals"].as_array().unwrap_or(&empty); // This could be empty, thus return None

    let special_str = match all_goals.len() {
        0 => "",
        _ => {
            let special = all_goals.last().unwrap();
            match special["period"].as_str().unwrap() {
                "OT" => "ot",
                "SO" => "so",
                _ => "",
            }
        }
    };

    let score = format!("{}-{}", home_score, away_score);
    let goals: &Vec<serde_json::Value> = game_json["goals"].as_array().unwrap();

    let goals = goals
        .into_iter()
        .map(|goal| {
            let raw_min = match goal["period"].as_str().unwrap() {
                "SO" => 65,
                _ => format_minute(
                    goal["min"].as_u64().unwrap(),
                    &goal["period"].as_str().unwrap(),
                ),
            };

            let scorer = goal["scorer"]["player"].as_str().unwrap();
            let scorer = scorer.split(" ").collect::<Vec<&str>>();
            let scorer = scorer[1..scorer.len()].to_vec();
            let scorer = scorer.join(" ");

            return Goal {
                scorer: scorer,
                minute: raw_min,
                finn: false,                   // @TODO: Figure this out
                finn_assist: String::from(""), // @TODO: Figure this out,
                team: goal["team"].to_string().replace("\"", ""),
                special: is_special(goal),
            };
        })
        .collect::<Vec<Goal>>();
    let game = Game {
        home: String::from(*home_team),
        away: String::from(*away_team),
        score: score.to_owned(),
        goals: goals,
        status: String::from(game_json["status"]["state"].as_str().unwrap()),
        special: String::from(special_str),
    };

    print_game(&game);
    println!();
    game
}

fn print_game(game: &Game) {
    let home_scores: Vec<&Goal> = game
        .goals
        .iter()
        .filter(|goal| goal.team == (&game).home && goal.minute != 65)
        .collect::<Vec<&Goal>>();
    let away_scores: Vec<&Goal> = game
        .goals
        .iter()
        .filter(|goal| goal.team == (&game).away && goal.minute != 65)
        .collect::<Vec<&Goal>>();

    let mut shootout_scorer = None;

    if game.special == "so" {
        shootout_scorer = Some(game.goals.iter().last().unwrap());
    }

    // Print header
    white!(
        "{:<15} {:>2} {:<15} {:<2} ",
        translate_team_name(&game.home[..]),
        '-',
        translate_team_name(&game.away[..]),
        ""
    );
    if game.status == "LIVE" {
        white_ln!("{:>6}", game.score);
    } else if game.status == "FINAL" {
        green_ln!("{:>6}", format!("{} {}", game.special, game.score));
    }

    // Print scores
    let score_iter = home_scores.into_iter().zip_longest(away_scores.into_iter());
    for pair in score_iter {
        match pair {
            Both(l, r) => print_full(l, r),
            Left(l) => print_left(l),
            Right(r) => print_right(r),
        }
    }

    // Game-winning shootout goal is always on its own line because
    // the game must be tied before it so it's safe to print it after everything.
    // If we later add assists by Finns, this needs to be rewritten.
    if let Some(so) = shootout_scorer {
        if so.team == game.home {
            print_left(so)
        } else {
            print_right(so)
        }
    }
}

fn print_full(home: &Goal, away: &Goal) {
    if home.special {
        magenta!("{:<15} {:>2} ", home.scorer, home.minute);
    } else if home.finn {
        green!("{:<15} {:>2} ", home.scorer, home.minute);
    } else {
        cyan!("{:<15} {:>2} ", home.scorer, home.minute);
    }

    if away.special {
        magenta_ln!("{:<15} {:>2}", away.scorer, away.minute);
    } else if away.finn {
        green_ln!("{:<15} {:>2}", away.scorer, away.minute);
    } else {
        cyan_ln!("{:<15} {:>2}", away.scorer, away.minute);
    }
}

fn print_left(home: &Goal) {
    if home.special {
        magenta_ln!("{:<15} {:>2}", home.scorer, home.minute);
    } else if home.finn {
        green_ln!("{:<15} {:>2}", home.scorer, home.minute);
    } else {
        cyan_ln!("{:<15} {:>2}", home.scorer, home.minute);
    }
}

fn print_right(away: &Goal) {
    if away.special {
        magenta_ln!(
            "{:<15} {:>2} {:<15} {:>2}",
            "",
            "",
            away.scorer,
            away.minute
        );
    } else if away.finn {
        green_ln!(
            "{:<15} {:>2} {:<15} {:>2}",
            "",
            "",
            away.scorer,
            away.minute
        );
    } else {
        cyan_ln!(
            "{:<15} {:>2} {:<15} {:>2}",
            "",
            "",
            away.scorer,
            away.minute
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minutes_are_converted_correctly() {
        assert_eq!(format_minute(3, "1"), 3);
        assert_eq!(format_minute(13, "2"), 33);
        assert_eq!(format_minute(5, "3"), 45);
        assert_eq!(format_minute(4, "OT"), 64);
        assert_eq!(format_minute(0, "1"), 0);
        assert_eq!(format_minute(0, "2"), 20);
        assert_eq!(format_minute(0, "3"), 40);
        assert_eq!(format_minute(0, "OT"), 60);
    }
}
