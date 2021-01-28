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
        .collect::<Vec<Option<Game>>>();

    _results.into_iter().for_each(|game| match game {
        Some(game) => print_game(&game),
        None => (),
    });

    Ok(())
}

fn format_minute(min: u64, period: &str) -> u64 {
    if period == "OT" {
        60 + min
    } else {
        let numeric_period: u64 = period.parse().unwrap();
        20 * (numeric_period - 1) + min
    }
}

fn is_special(goal: &serde_json::Value) -> bool {
    let period = goal["period"].as_str();
    match period {
        Some(period) => {
            let is_ot = period == "OT";
            let is_so = period == "SO";
            let is_playoff_ot = match period.parse::<u64>() {
                Ok(period) => period >= 4,
                Err(_) => false,
            };
            is_ot || is_so || is_playoff_ot
        }
        None => false,
    }
}

fn parse_game(game_json: &serde_json::Value) -> Option<Game> {
    if (&game_json["teams"]).is_null() {
        return None;
    }
    let home_team = &game_json["teams"]["home"]["abbreviation"].as_str().unwrap();
    let away_team = &game_json["teams"]["away"]["abbreviation"].as_str().unwrap();

    if (&game_json["scores"]).is_null() {
        return None;
    }

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

    Some(game)
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
    } else if game.status == "POSTPONED" {
        white_ln!("{:>6}", "POSTP.");
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
    println!();
}

fn print_full(home: &Goal, away: &Goal) {
    let home_message = format!("{:<15} {:>2} ", home.scorer, home.minute);
    if home.special {
        magenta!("{}", home_message);
    } else if home.finn {
        green!("{}", home_message);
    } else {
        cyan!("{}", home_message);
    }

    let away_message = format!("{:<15} {:>2}", away.scorer, away.minute);
    if away.special {
        magenta_ln!("{}", away_message);
    } else if away.finn {
        green_ln!("{}", away_message);
    } else {
        cyan_ln!("{}", away_message);
    }
}

fn print_left(home: &Goal) {
    let message = format!("{:<15} {:>2}", home.scorer, home.minute);
    if home.special {
        magenta_ln!("{}", message);
    } else if home.finn {
        green_ln!("{}", message);
    } else {
        cyan_ln!("{}", message);
    }
}

fn print_right(away: &Goal) {
    let message = format!(
        "{:<15} {:>2} {:<15} {:>2}",
        "", "", away.scorer, away.minute
    );
    if away.special {
        magenta_ln!("{}", message);
    } else if away.finn {
        green_ln!("{}", message);
    } else {
        cyan_ln!("{}", message);
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
        assert_eq!(format_minute(12, "4"), 72);
        assert_eq!(format_minute(5, "5"), 85);
        assert_eq!(format_minute(5, "6"), 105);
        assert_eq!(format_minute(4, "OT"), 64);
        assert_eq!(format_minute(0, "1"), 0);
        assert_eq!(format_minute(0, "2"), 20);
        assert_eq!(format_minute(0, "3"), 40);
        assert_eq!(format_minute(0, "OT"), 60);
    }

    #[test]
    fn is_special_works() -> serde_json::Result<()> {
        let first = r#"{ "team": "CHI", "period": "1" }"#;
        let second = r#"{ "team": "CHI", "period": "2" }"#;
        let third = r#"{ "team": "CHI", "period": "3" }"#;
        let overtime = r#"{ "team": "CHI", "period": "OT" }"#;
        let shootout = r#"{ "team": "CHI", "period": "SO" }"#;
        let missing_data = r#"{ "team": "CHI" }"#;
        let playoff_ot = r#"{ "team": "CHI", "period": "4" }"#;
        let playoff_ot_2 = r#"{ "team": "CHI", "period": "10" }"#;

        let goal1: serde_json::Value = serde_json::from_str(&first)?;
        let goal2: serde_json::Value = serde_json::from_str(&second)?;
        let goal3: serde_json::Value = serde_json::from_str(&third)?;
        let goal4: serde_json::Value = serde_json::from_str(&overtime)?;
        let goal5: serde_json::Value = serde_json::from_str(&shootout)?;
        let goal6: serde_json::Value = serde_json::from_str(&missing_data)?;
        let goal7: serde_json::Value = serde_json::from_str(&playoff_ot)?;
        let goal8: serde_json::Value = serde_json::from_str(&playoff_ot_2)?;

        assert_eq!(is_special(&goal1), false);
        assert_eq!(is_special(&goal2), false);
        assert_eq!(is_special(&goal3), false);
        assert_eq!(is_special(&goal4), true);
        assert_eq!(is_special(&goal5), true);
        assert_eq!(is_special(&goal6), false);
        assert_eq!(is_special(&goal7), true);
        assert_eq!(is_special(&goal8), true);

        Ok(())
    }

    #[test]
    fn it_parses_full_live_game_data_correctly() -> serde_json::Result<()> {
        let test_game = serde_json::from_str(
            r#"{"status":{"state":"LIVE","progress":{"currentPeriod":3,"currentPeriodOrdinal":"3rd","currentPeriodTimeRemaining":{"min":12,"sec":21,"pretty":"12:21"}}},"startTime":"2021-01-23T19:00:00Z","goals":[{"team":"TBL","period":"1","scorer":{"player":"Victor Hedman","seasonTotal":1},"assists":[{"player":"Mitchell Stephens","seasonTotal":1},{"player":"Alexander Volkov","seasonTotal":1}],"min":4,"sec":10},{"team":"CBJ","period":"1","scorer":{"player":"Nick Foligno","seasonTotal":3},"assists":[{"player":"Cam Atkinson","seasonTotal":2},{"player":"Michael Del Zotto","seasonTotal":4}],"min":4,"sec":27},{"team":"CBJ","period":"1","scorer":{"player":"Mikhail Grigorenko","seasonTotal":1},"assists":[{"player":"Kevin Stenlund","seasonTotal":1},{"player":"Nathan Gerbe","seasonTotal":1}],"min":10,"sec":3},{"team":"CBJ","period":"1","scorer":{"player":"Vladislav Gavrikov","seasonTotal":1},"assists":[{"player":"Liam Foudy","seasonTotal":2},{"player":"Eric Robinson","seasonTotal":1}],"min":19,"sec":1},{"team":"TBL","period":"1","scorer":{"player":"Ondrej Palat","seasonTotal":3},"assists":[{"player":"Brayden Point","seasonTotal":3},{"player":"Victor Hedman","seasonTotal":4}],"min":19,"sec":46,"strength":"PPG"},{"team":"CBJ","period":"3","scorer":{"player":"Zach Werenski","seasonTotal":1},"assists":[{"player":"Alexandre Texier","seasonTotal":2},{"player":"Boone Jenner","seasonTotal":2}],"min":6,"sec":34}],"scores":{"TBL":2,"CBJ":4},"teams":{"away":{"abbreviation":"TBL","id":14,"locationName":"Tampa Bay","shortName":"Tampa Bay","teamName":"Lightning"},"home":{"abbreviation":"CBJ","id":29,"locationName":"Columbus","shortName":"Columbus","teamName":"Blue Jackets"}},"preGameStats":{"records":{"TBL":{"wins":3,"losses":0,"ot":0},"CBJ":{"wins":1,"losses":2,"ot":2}}},"currentStats":{"records":{"TBL":{"wins":3,"losses":0,"ot":0},"CBJ":{"wins":1,"losses":2,"ot":2}},"streaks":{"TBL":{"type":"WINS","count":3},"CBJ":{"type":"OT","count":2}},"standings":{"TBL":{"divisionRank":"1","leagueRank":"1"},"CBJ":{"divisionRank":"7","leagueRank":"24"}}}}"#,
        )?;

        let parsed_game = parse_game(&test_game).unwrap();

        assert_eq!(parsed_game.home, "CBJ");
        assert_eq!(parsed_game.away, "TBL");
        assert_eq!(parsed_game.score, "4-2");
        assert_eq!(parsed_game.goals.len(), 6);
        assert_eq!(parsed_game.status, "LIVE");
        assert_eq!(parsed_game.special, "");

        Ok(())
    }

    #[test]
    fn it_parses_full_overtime_game_data_correctly() -> serde_json::Result<()> {
        let test_game = serde_json::from_str(
            r#"
            {
                "status":{
                    "state":"FINAL"
                },
                "startTime":"2021-01-23T19:00:00Z",
                "goals":[
                    {
                        "team":"TOR",
                        "period":"1",
                        "scorer":{
                            "player":"Mitch Marner",
                            "seasonTotal":1
                        },
                        "assists":[
                            {
                                "player":"Mitchell Stephens",
                                "seasonTotal":1
                            },
                            {
                                "player":"Alexander Volkov",
                                "seasonTotal":1
                            }
                        ],
                        "min":4,
                        "sec":10
                    },
                    {
                        "team":"PIT",
                        "period":"3",
                        "scorer":{
                            "player":"Sidney Crosby",
                            "seasonTotal":3
                        },
                        "assists":[
                            {
                                "player":"Evgeni Malkin",
                                "seasonTotal":2
                            }
                        ],
                        "min":4,
                        "sec":27
                    },
                    {
                        "team":"PIT",
                        "period":"OT",
                        "scorer":{
                            "player":"Sidney Crosby",
                            "seasonTotal":4
                        },
                        "assists":[],
                        "min":3,
                        "sec":0
                    }],
                    "scores":{
                        "PIT":2,"TOR":1
                    },
                    "teams":{
                        "away":{
                            "abbreviation":"PIT",
                            "id":14,
                            "locationName":"Pittsburgh",
                            "shortName":"Pittsburgh",
                            "teamName":"Penguins"
                        },
                        "home":{
                            "abbreviation":"TOR",
                            "id":29,
                            "locationName":"Toronto",
                            "shortName":"Toronto",
                            "teamName":"Maple Leafs"
                        }
                    },
                    "preGameStats":{"records":{"PIT":{"wins":3,"losses":0,"ot":0},"TOR":{"wins":1,"losses":2,"ot":2}}},
                    "currentStats":{"records":{"PIT":{"wins":4,"losses":0,"ot":0},"TOR":{"wins":1,"losses":2,"ot":3}},
                    "streaks":{"PIT":{"type":"WINS","count":3},"TOR":{"type":"OT","count":2}},
                    "standings":{
                        "PIT":{"divisionRank":"1","leagueRank":"1"},
                        "CBJ":{"divisionRank":"7","leagueRank":"24"}
                    }
                }
            }"#,
        )?;

        let parsed_game = parse_game(&test_game).unwrap();

        assert_eq!(parsed_game.home, "TOR");
        assert_eq!(parsed_game.away, "PIT");
        assert_eq!(parsed_game.score, "1-2");
        assert_eq!(parsed_game.goals.len(), 3);
        assert_eq!(parsed_game.status, "FINAL");
        assert_eq!(parsed_game.special, "ot");

        Ok(())
    }

    #[test]
    fn it_parses_a_game_with_no_goals_correctly() -> serde_json::Result<()> {
        let test_game = serde_json::from_str(
            r#"
            {
                "status":{
                    "state":"LIVE"
                },
                "startTime":"2021-01-23T19:00:00Z",
                "goals":[],
                    "scores":{
                        "PIT":0,"TOR":0
                    },
                    "teams":{
                        "away":{
                            "abbreviation":"PIT",
                            "id":14,
                            "locationName":"Pittsburgh",
                            "shortName":"Pittsburgh",
                            "teamName":"Penguins"
                        },
                        "home":{
                            "abbreviation":"TOR",
                            "id":29,
                            "locationName":"Toronto",
                            "shortName":"Toronto",
                            "teamName":"Maple Leafs"
                        }
                    },
                    "preGameStats":{"records":{"PIT":{"wins":3,"losses":0,"ot":0},"TOR":{"wins":1,"losses":2,"ot":2}}},
                    "currentStats":{"records":{"PIT":{"wins":4,"losses":0,"ot":0},"TOR":{"wins":1,"losses":2,"ot":3}},
                    "streaks":{"PIT":{"type":"WINS","count":3},"TOR":{"type":"OT","count":2}},
                    "standings":{
                        "PIT":{"divisionRank":"1","leagueRank":"1"},
                        "CBJ":{"divisionRank":"7","leagueRank":"24"}
                    }
                }
            }"#,
        )?;

        let parsed_game = parse_game(&test_game).unwrap();

        assert_eq!(parsed_game.home, "TOR");
        assert_eq!(parsed_game.away, "PIT");
        assert_eq!(parsed_game.score, "0-0");
        assert_eq!(parsed_game.goals.len(), 0);
        assert_eq!(parsed_game.status, "LIVE");
        assert_eq!(parsed_game.special, "");

        Ok(())
    }

    #[test]
    fn it_parses_missing_teams_data_correctly() -> serde_json::Result<()> {
        let test_game = serde_json::from_str(
            r#"{"status":{"state":"LIVE","progress":{"currentPeriod":3,"currentPeriodOrdinal":"3rd","currentPeriodTimeRemaining":{"min":12,"sec":21,"pretty":"12:21"}}},"startTime":"2021-01-23T19:00:00Z","goals":[{"team":"TBL","period":"1","scorer":{"player":"Victor Hedman","seasonTotal":1},"assists":[{"player":"Mitchell Stephens","seasonTotal":1},{"player":"Alexander Volkov","seasonTotal":1}],"min":4,"sec":10},{"team":"CBJ","period":"1","scorer":{"player":"Nick Foligno","seasonTotal":3},"assists":[{"player":"Cam Atkinson","seasonTotal":2},{"player":"Michael Del Zotto","seasonTotal":4}],"min":4,"sec":27},{"team":"CBJ","period":"1","scorer":{"player":"Mikhail Grigorenko","seasonTotal":1},"assists":[{"player":"Kevin Stenlund","seasonTotal":1},{"player":"Nathan Gerbe","seasonTotal":1}],"min":10,"sec":3},{"team":"CBJ","period":"1","scorer":{"player":"Vladislav Gavrikov","seasonTotal":1},"assists":[{"player":"Liam Foudy","seasonTotal":2},{"player":"Eric Robinson","seasonTotal":1}],"min":19,"sec":1},{"team":"TBL","period":"1","scorer":{"player":"Ondrej Palat","seasonTotal":3},"assists":[{"player":"Brayden Point","seasonTotal":3},{"player":"Victor Hedman","seasonTotal":4}],"min":19,"sec":46,"strength":"PPG"},{"team":"CBJ","period":"3","scorer":{"player":"Zach Werenski","seasonTotal":1},"assists":[{"player":"Alexandre Texier","seasonTotal":2},{"player":"Boone Jenner","seasonTotal":2}],"min":6,"sec":34}],"scores":{"TBL":2,"CBJ":4},"preGameStats":{"records":{"TBL":{"wins":3,"losses":0,"ot":0},"CBJ":{"wins":1,"losses":2,"ot":2}}},"currentStats":{"records":{"TBL":{"wins":3,"losses":0,"ot":0},"CBJ":{"wins":1,"losses":2,"ot":2}},"streaks":{"TBL":{"type":"WINS","count":3},"CBJ":{"type":"OT","count":2}},"standings":{"TBL":{"divisionRank":"1","leagueRank":"1"},"CBJ":{"divisionRank":"7","leagueRank":"24"}}}}"#,
        )?;

        let parsed_game = parse_game(&test_game);
        assert_eq!(parsed_game.is_some(), false);

        Ok(())
    }
}
