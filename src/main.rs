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
use atty::Stream;
use dirs::home_dir;
use itertools::{EitherOrBoth::*, Itertools};
use reqwest::Error;
use std::collections::HashMap;
use std::fs::File;
use std::io::Error as StdError;
use std::io::Read;
use std::process;
use structopt::StructOpt;

const SHOOTOUT_MINUTE: u64 = 65;

mod api_types;
use api_types::{APIResponse, GameResponse, GoalResponse};

#[derive(Debug)]
struct Goal {
    scorer: String,
    assists: Vec<String>,
    minute: u64,
    special: bool,
    team: String,
}

#[derive(Debug)]
struct Stat {
    goals: u64,
    assists: u64,
}

#[derive(Debug)]
struct Game {
    home: String,
    away: String,
    score: String,
    goals: Vec<Goal>,
    status: String,
    special: String,
    playoff_series: Option<HashMap<String, serde_json::Value>>,
}

#[derive(StructOpt, Debug)]
/// Display live or previous NHL match results on command line
///
/// Homepage: https://hamatti.github.io/nhl-235/
///
/// Open source under MIT license
struct Cli {
    #[structopt(long)]
    #[structopt(help = "Current version")]
    version: bool,
    #[structopt(long)]
    #[structopt(help = "Disable terminal colors")]
    nocolors: bool,
    #[structopt(long)]
    #[structopt(
        help = "Highlight players based on $HOME/.235.config file. If --nocolors is enabled, does nothing"
    )]
    highlight: bool,
    #[structopt(long)]
    #[structopt(
        help = "Display stats (goals + assists) for players defined in $HOME/.235.config file."
    )]
    stats: bool,
}

fn main() {
    let args = Cli::from_args();
    // Using an inverse here because default is colors enabled
    // and I want to keep the API easier to read down the line,
    // hence colors need to be enabled rather than disabled
    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let use_colors = !args.nocolors;
    let highlights = read_highlight_config().unwrap_or_default();

    let show_stats = args.stats;
    let show_highlights: bool = args.highlight;

    match fetch_games() {
        Ok(scores) => {
            let parsed_games = parse_games(scores);
            print_games(
                parsed_games,
                use_colors,
                &highlights,
                show_highlights,
                show_stats,
            );
        }
        Err(err) => {
            handle_request_error(err);
        }
    };
}

fn read_highlight_config() -> Result<Vec<String>, StdError> {
    let mut config_file = home_dir().unwrap();
    config_file.push(".235.config");

    let mut file = File::open(config_file.as_path())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let highlights: Vec<String> = contents
        .split('\n')
        .map(str::to_string)
        .filter(|s| s != "")
        .collect();

    Ok(highlights)
}

fn handle_request_error(e: reqwest::Error) {
    if e.is_connect() {
        println!("ERROR: Can't connect to the API. It might be because your Internet connection is down.");
        process::exit(1);
    } else if e.is_timeout() {
        println!("ERROR: API timed out. Try again later.");
        process::exit(1);
    } else if e.is_decode() {
        println!("ERROR: API returned malformed data. Try again later.");
        println!("{:?}", e);
        process::exit(1);
    } else {
        println!("ERROR: Unknown error.");
        println!("{:?}", e);
        process::exit(1);
    }
}

fn translate_team_name(abbr: &str) -> String {
    let city = match abbr {
        "BOS" => "Boston",
        "BUF" => "Buffalo",
        "NJD" => "New Jersey",
        "NYI" => "NY Islanders", // Islanders is named like this to differentiate two New York teams
        "NYR" => "NY Rangers",   // Rangers is named like this to differentiate two New York teams
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
        "SEA" => "Seattle",
        _ => "[unknown]",
    };

    String::from(city)
}

#[tokio::main]
async fn fetch_games() -> Result<APIResponse, Error> {
    let request_url = String::from("https://nhl-score-api.herokuapp.com/api/scores/latest");
    let response = reqwest::get(&request_url).await?;
    let scores: APIResponse = response.json().await?;

    Ok(scores)
}

/// Transforms a JSON structure of multiple games into
/// a vector of Option<Game> so they can be processed by
/// other parts of the application
fn parse_games(scores: APIResponse) -> Vec<Option<Game>> {
    let games = scores.games;

    games
        .iter()
        .map(|game| parse_game(game))
        .collect::<Vec<Option<Game>>>()
}

/// Handler function to print multiple Games
fn print_games(
    games: Vec<Option<Game>>,
    use_colors: bool,
    highlights: &[String],
    show_highlights: bool,
    show_stats: bool,
) {
    match games.len() {
        0 => println!("No games today."),
        _ => {
            games.into_iter().for_each(|game| match game {
                Some(game) => {
                    print_game(&game, use_colors, &highlights, show_highlights, show_stats)
                }
                None => (),
            });
        }
    }
}

/// Transforms a combination of min (between 0 and 19) and
/// period ("OT", "SO" or number > 0 in number form)
/// into a numeric minute given 20 minute periods
fn format_minute(min: u64, period: &str) -> u64 {
    if period == "OT" {
        60 + min
    } else {
        let period: u64 = period.parse().unwrap();
        20 * (period - 1) + min
    }
}

/// Returns true if the goal scored was done in
/// overtime or in a shootout
fn is_special(goal: &GoalResponse) -> bool {
    match goal.period.parse::<u64>() {
        Ok(period) => period >= 4,
        Err(_) => true,
    }
}

/// Transforms a JSON structure of an individual game into a Game
fn parse_game(game_json: &GameResponse) -> Option<Game> {
    let home_team = &game_json.teams.home.abbreviation;
    let away_team = &game_json.teams.away.abbreviation;

    let home_score = &game_json.scores[home_team];
    let away_score = &game_json.scores[away_team];

    let empty_vec: &Vec<GoalResponse> = &Vec::<GoalResponse>::new();

    let all_goals = match &game_json.goals {
        Some(goals) => goals,
        None => &empty_vec,
    };

    let special = match all_goals.last() {
        None => "",
        Some(last_goal) => {
            let period = &last_goal.period;
            match period.as_str() {
                "1" | "2" | "3" => "",
                "OT" => "ot",
                "SO" => "so",
                // The default case is "ot" because the only ones
                // with chars should be OT and SO and this matches
                // Any digit larger than 3.
                // If other periods occur, new arms should be added
                _ => "ot",
            }
        }
    };

    let goals: &Vec<GoalResponse> = all_goals;

    let goals = goals
        .into_iter()
        .map(|goal| {
            let minute = match goal.period.as_str() {
                "SO" => SHOOTOUT_MINUTE,
                _ => format_minute(goal.min.unwrap(), &goal.period),
            };

            let scorer = extract_scorer_name(&goal.scorer.player);
            let assists = &goal
                .assists
                .as_ref()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|assist| extract_scorer_name(&assist.player))
                .collect::<Vec<String>>();

            return Goal {
                scorer: scorer,
                assists: assists.to_vec(),
                minute: minute,
                team: goal.team.replace("\"", ""),
                special: is_special(goal),
            };
        })
        .collect::<Vec<Goal>>();

    let score = format!("{}-{}", home_score, away_score);
    let game = Game {
        home: String::from(home_team),
        away: String::from(away_team),
        score: score.to_owned(),
        goals: goals,
        status: String::from(&game_json.status.state),
        special: String::from(special),
        playoff_series: game_json.current_stats.playoff_series.clone(),
    };

    Some(game)
}

/// Attempts to return player's last name
/// by removing the first part of player's name.
///
/// This is not always correct since a player
/// can have multiple first names but it's a
/// tradeoff since we don't have data on that
/// and full name would be too long
fn extract_scorer_name(name: &str) -> String {
    let name = name.split(" ").collect::<Vec<&str>>();
    let name = name[1..name.len()].to_vec();
    name.join(" ")
}

fn print_game(
    game: &Game,
    use_colors: bool,
    highlights: &[String],
    show_highlights: bool,
    show_stats: bool,
) {
    let home_scores: Vec<&Goal> = game
        .goals
        .iter()
        .filter(|goal| {
            goal.team == game.home
                && (goal.minute != SHOOTOUT_MINUTE
                    || goal.minute == SHOOTOUT_MINUTE && game.special == "ot")
        })
        .collect::<Vec<&Goal>>();
    let away_scores: Vec<&Goal> = game
        .goals
        .iter()
        .filter(|goal| {
            goal.team == game.away
                && (goal.minute != SHOOTOUT_MINUTE
                    || goal.minute == SHOOTOUT_MINUTE && game.special == "ot")
        })
        .collect::<Vec<&Goal>>();

    let mut shootout_scorer = None;

    if game.special == "so" {
        shootout_scorer = Some(game.goals.iter().last().unwrap());
    }

    // Print header
    if atty::is(Stream::Stdout) && use_colors {
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
    } else {
        print!(
            "{:<15} {:>2} {:<15} {:<2} ",
            translate_team_name(&game.home[..]),
            '-',
            translate_team_name(&game.away[..]),
            ""
        );
        if game.status == "LIVE" {
            println!("{:>6}", game.score);
        } else if game.status == "FINAL" {
            println!("{:>6}", format!("{} {}", game.special, game.score));
        } else if game.status == "POSTPONED" {
            println!("{:>6}", "POSTP.");
        }
    }

    // Print scores
    let score_pairs = home_scores.iter().zip_longest(away_scores.iter());
    for pair in score_pairs {
        match pair {
            Both(home, away) => {
                print_both_goals(home, away, use_colors, highlights, show_highlights)
            }
            Left(home) => print_home_goal(home, use_colors, highlights, show_highlights),
            Right(away) => print_away_goal(away, use_colors, highlights, show_highlights),
        }
    }

    // Game-winning shootout goal is always on its own line because
    // the game must be tied before it so it's safe to print it after everything.
    // If we later add assists by Finns, this needs to be rewritten.
    if let Some(shootout_goal) = shootout_scorer {
        if shootout_goal.team == game.home {
            print_home_goal(shootout_goal, use_colors, highlights, show_highlights)
        } else {
            print_away_goal(shootout_goal, use_colors, highlights, show_highlights)
        }
    }
    println!();

    if show_stats {
        print_stats(&game.goals, &highlights, show_highlights);
    }

    match &game.playoff_series {
        Some(playoff_series) => {
            let series_wins = &playoff_series["wins"];
            let home_wins = &series_wins[&game.home];
            let away_wins = &series_wins[&game.away];

            if atty::is(Stream::Stdout) && use_colors {
                yellow_ln!("Series {}-{}", home_wins, away_wins);
            } else {
                println!("Series {}-{}", home_wins, away_wins);
            }
            println!();
        }
        None => (),
    }
}

fn print_both_goals(
    home: &Goal,
    away: &Goal,
    use_colors: bool,
    highlights: &[String],
    show_highlights: bool,
) {
    let home_message = format!("{:<15} {:>2} ", home.scorer, home.minute);
    if atty::is(Stream::Stdout) && use_colors {
        if home.special {
            magenta!("{}", home_message);
        } else if show_highlights && highlights.contains(&home.scorer) {
            yellow!("{}", home_message);
        } else {
            cyan!("{}", home_message);
        }
    } else {
        print!("{}", home_message);
    }

    let away_message = format!("{:<15} {:>2}", away.scorer, away.minute);
    if atty::is(Stream::Stdout) && use_colors {
        if away.special {
            magenta_ln!("{}", away_message);
        } else if show_highlights && highlights.contains(&away.scorer) {
            yellow_ln!("{}", away_message);
        } else {
            cyan_ln!("{}", away_message);
        }
    } else {
        println!("{}", away_message);
    }
}

fn print_home_goal(home: &Goal, use_colors: bool, highlights: &[String], show_highlights: bool) {
    let message = format!("{:<15} {:>2}", home.scorer, home.minute);
    if atty::is(Stream::Stdout) && use_colors {
        if home.special {
            magenta_ln!("{}", message);
        } else if show_highlights && highlights.contains(&home.scorer) {
            yellow_ln!("{}", message);
        } else {
            cyan_ln!("{}", message);
        }
    } else {
        println!("{}", message);
    }
}

fn print_away_goal(away: &Goal, use_colors: bool, highlights: &[String], show_highlights: bool) {
    let message = format!(
        "{:<15} {:>2} {:<15} {:>2}",
        "", "", away.scorer, away.minute
    );
    if atty::is(Stream::Stdout) && use_colors {
        if away.special {
            magenta_ln!("{}", message);
        } else if show_highlights && highlights.contains(&away.scorer) {
            yellow_ln!("{}", message);
        } else {
            cyan_ln!("{}", message);
        }
    } else {
        println!("{}", message);
    }
}

fn count_stats(goal: &Goal, stats: &mut HashMap<String, Stat>, highlights: &[String]) {
    if highlights.contains(&goal.scorer) {
        stats
            .entry(String::from(&goal.scorer))
            .and_modify(|stat| stat.goals += 1)
            .or_insert(Stat {
                goals: 1,
                assists: 0,
            });
    }

    goal.assists.iter().for_each(|assist| {
        if highlights.contains(&assist) {
            stats
                .entry(String::from(assist))
                .and_modify(|stat| stat.assists += 1)
                .or_insert(Stat {
                    goals: 0,
                    assists: 1,
                });
        }
    })
}

fn print_stats(goals: &Vec<Goal>, highlights: &[String], show_highlights: bool) {
    let mut stats: HashMap<String, Stat> = HashMap::new();

    goals.iter().for_each(|goal| {
        count_stats(&goal, &mut stats, &highlights);
    });

    if stats.is_empty() {
        return;
    }

    let mut stats_messages: Vec<String> = Vec::new();
    for (name, stats) in stats.iter() {
        let sub_message = format!(
            "{} {}+{}",
            name,
            &stats.goals.to_string(),
            &stats.assists.to_string()
        );
        stats_messages.push(sub_message);
    }
    let message: String = format!("({})", stats_messages.join(", "));

    if show_highlights {
        yellow_ln!("{}", message);
    } else {
        println!("{}", message);
    }
    println!();
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
        let first =
            r#"{ "team": "CHI", "period": "1", "scorer": { "player": "_", "seasonTotal": 10} }"#;
        let second =
            r#"{ "team": "CHI", "period": "2", "scorer": { "player": "_", "seasonTotal": 10}  }"#;
        let third =
            r#"{ "team": "CHI", "period": "3", "scorer": { "player": "_", "seasonTotal": 10}  }"#;
        let overtime =
            r#"{ "team": "CHI", "period": "OT", "scorer": { "player": "_", "seasonTotal": 10}  }"#;
        let shootout =
            r#"{ "team": "CHI", "period": "SO", "scorer": { "player": "_", "seasonTotal": 10}  }"#;
        let playoff_ot =
            r#"{ "team": "CHI", "period": "4", "scorer": { "player": "_", "seasonTotal": 10}  }"#;
        let playoff_ot_2 =
            r#"{ "team": "CHI", "period": "10", "scorer": { "player": "_", "seasonTotal": 10}  }"#;
        let wrong_data =
            r#"{ "team": "CHI", "period": "SP", "scorer": { "player": "_", "seasonTotal": 10}  }"#;

        let goal1: GoalResponse = serde_json::from_str(&first)?;
        let goal2: GoalResponse = serde_json::from_str(&second)?;
        let goal3: GoalResponse = serde_json::from_str(&third)?;
        let goal4: GoalResponse = serde_json::from_str(&overtime)?;
        let goal5: GoalResponse = serde_json::from_str(&shootout)?;
        let goal6: GoalResponse = serde_json::from_str(&playoff_ot)?;
        let goal7: GoalResponse = serde_json::from_str(&playoff_ot_2)?;
        let goal8: GoalResponse = serde_json::from_str(&wrong_data)?;

        assert_eq!(is_special(&goal1), false);
        assert_eq!(is_special(&goal2), false);
        assert_eq!(is_special(&goal3), false);
        assert_eq!(is_special(&goal4), true);
        assert_eq!(is_special(&goal5), true);
        assert_eq!(is_special(&goal6), true);
        assert_eq!(is_special(&goal7), true);
        // I haven't yet really decided what this should be but
        // important thing is that it does not crash the app
        assert_eq!(is_special(&goal8), true);

        Ok(())
    }

    #[test]
    fn it_parses_full_live_game_data_correctly() -> serde_json::Result<()> {
        let test_game: GameResponse = serde_json::from_str(
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
    fn it_parses_a_playoffs_game_with_overtime_correctly() -> serde_json::Result<()> {
        let test_game = serde_json::from_str(
            r#"
            {
                "status":{
                    "state":"FINAL"
                },
                "startTime":"2021-01-23T19:00:00Z",
                "goals":[{
                    "team":"PIT",
                    "period":"4",
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
                }],
                    "scores":{
                        "PIT":1,"TOR":0
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
        assert_eq!(parsed_game.score, "0-1");
        assert_eq!(parsed_game.goals.len(), 1);
        assert_eq!(parsed_game.status, "FINAL");
        assert_eq!(parsed_game.special, "ot");

        Ok(())
    }

    #[test]
    fn it_extracts_player_name_correctly() {
        assert_eq!(extract_scorer_name("Olli Maatta"), String::from("Maatta"));
        assert_eq!(
            extract_scorer_name("James van Riemsdyk"),
            String::from("van Riemsdyk")
        );
    }
}
