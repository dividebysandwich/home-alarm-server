use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use enum_stringify::EnumStringify;
use std::{fs::File, io::{Read, Write}, sync::{Arc, Mutex}};

const PRIMARY_CODE : &str = "12345";
const SECONDARY_CODE : &str = "00000000";
const HOME_LATITUDE : f64 = 48.0000;
const HOME_LONGITUDE : f64 = 15.0000;
const SCRIPT_LIGHT_STAIRCASE : &str = "./light_staircase.sh";
const SCRIPT_LIGHT_DESKLAMP : &str = "./light_desklamp.sh";
const SCRIPT_ARM_AWAY : &str = "./armedaway.sh";
const SCRIPT_ARM_QUIET : &str = "./armedawayquiet.sh";
const SCRIPT_DISARM : &str = "./disarmed.sh";
const SCRIPT_DISARM_QUIET : &str = "./disarmedquiet.sh";
const SCRIPT_ALARM_TRIGGER : &str = "./alarm.sh";
const SAVESTATE_FILE : &str = "alarmstate.txt";

#[derive(PartialEq, EnumStringify, Copy, Clone, Debug)]
enum AlarmState {
    #[enum_stringify(rename = "disarmed")]
    Disarmed,
    #[enum_stringify(rename = "armed_home")]
    ArmedHome,
    #[enum_stringify(rename = "armed_away")]
    ArmedAway,
    #[enum_stringify(rename = "alarm")]
    Alarm,
}

// State to keep track of the last number received
struct AppState {
    current_state: Mutex<AlarmState>,
}

async fn trigger_away(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(mut current_state) => {
            if *current_state == AlarmState::ArmedAway || *current_state == AlarmState::ArmedHome {
                *current_state = AlarmState::Alarm;
                execute_script(SCRIPT_ALARM_TRIGGER);
                persist_alarm_state(*current_state);
            }
            HttpResponse::Ok().body("Success")
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn trigger_home(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(mut current_state) => {
            if *current_state == AlarmState::ArmedHome {
                *current_state = AlarmState::Alarm;
                execute_script(SCRIPT_ALARM_TRIGGER);
                persist_alarm_state(*current_state);
            }
            HttpResponse::Ok().body("Success")
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn trigger_motion_downstairs(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(current_state) => {
            if is_sun_set() {
                if *current_state == AlarmState::Disarmed {
                    execute_script(SCRIPT_LIGHT_STAIRCASE);
                }
            }
            HttpResponse::Ok().body("Success")
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn trigger_motion_upstairs(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(current_state) => {
            if is_sun_set() {
                if *current_state == AlarmState::Disarmed {
                    execute_script(SCRIPT_LIGHT_STAIRCASE);
                }
                else if *current_state == AlarmState::ArmedAway || *current_state == AlarmState::ArmedHome {
                    execute_script(SCRIPT_LIGHT_DESKLAMP);
                }
            }
            HttpResponse::Ok().body("Success")
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn get_status(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(current_state) => {
            HttpResponse::Ok().body(current_state.to_string())
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn arm_disarm_toggle(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(mut current_state) => {
            if *current_state ==  AlarmState::Disarmed {
                *current_state = AlarmState::ArmedAway;
                execute_script(SCRIPT_ARM_AWAY);
            } else {
                *current_state = AlarmState::Disarmed;
                execute_script(SCRIPT_DISARM);
            }
            persist_alarm_state(*current_state);
            HttpResponse::Ok().body("Success")
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn arm_disarm_quiet_toggle(data: web::Data<Arc<AppState>>) -> impl Responder {
    match data.current_state.lock() {
        Ok(mut current_state) => {
            if *current_state ==  AlarmState::Disarmed {
                *current_state = AlarmState::ArmedHome;
                execute_script(SCRIPT_ARM_QUIET);
            } else {
                *current_state = AlarmState::Disarmed;
                execute_script(SCRIPT_DISARM_QUIET);
            }
            persist_alarm_state(*current_state);
            HttpResponse::Ok().body("Success")
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

// Write current_state to alarmstate.txt
fn persist_alarm_state(current_state: AlarmState) {
    let mut file = File::create(SAVESTATE_FILE).unwrap();
    file.write_all(current_state.to_string().as_bytes()).unwrap();
    println!("Current alarm state: {}", current_state.to_string());
}

// Load current_state from alarmstate.txt. If any error occurs, assume we're disarmed
fn load_alarm_state() -> AlarmState {
    match File::open(SAVESTATE_FILE) {
        Ok(mut file) => {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => {
                    match contents.trim() {
                        "disarmed" => AlarmState::Disarmed,
                        "armed_home" => AlarmState::ArmedHome,
                        "armed_away" => AlarmState::ArmedAway,
                        "alarm" => AlarmState::Alarm,
                        _ => AlarmState::Disarmed,
                    }        
                },
                Err(_) => {
                    AlarmState::Disarmed
                }
            }
        },
        Err(_) => {
            AlarmState::Disarmed
        }
    }


}

fn execute_script( scriptfile : &str) {
    match std::process::Command::new(scriptfile).output() {
        Ok(_output) => {
//            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
//            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        },
        Err(e) => {
            println!("Error executing script {}: {}", scriptfile, e);
        }
    }
}

fn is_sun_set() -> bool {
    // Get the current unix time
    let unixtime = suncalc::Timestamp(chrono::Utc::now().timestamp() as i64);

    // Get the sun position
    let pos = suncalc::get_position(unixtime,HOME_LATITUDE, HOME_LONGITUDE);
    let alt = pos.altitude.to_degrees();
//    println!("Sun position: azimuth: {}, altitude: {}", az, alt);

    // Sun is set if altitude is negative
    return alt < 0.0;
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    
    // Create the shared state by loading it from a file
    let app_state = Arc::new(AppState {
        current_state: Mutex::new(load_alarm_state()),
    });

    // Start the web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .route(format!("/code/{}",PRIMARY_CODE).as_str(), web::get().to(arm_disarm_toggle))
            .route(format!("/code/{}",SECONDARY_CODE).as_str(), web::get().to(arm_disarm_toggle))
            .route(format!("/codeQuiet/{}",PRIMARY_CODE).as_str(), web::get().to(arm_disarm_quiet_toggle))
            .route(format!("/codeQuiet/{}",SECONDARY_CODE).as_str(), web::get().to(arm_disarm_quiet_toggle))
            .route("/trigger_away", web::get().to(trigger_away))
            .route("/trigger_home", web::get().to(trigger_home))
            .route("/trigger_motion_downstairs", web::get().to(trigger_motion_downstairs))
            .route("/trigger_motion_upstairs", web::get().to(trigger_motion_upstairs))
            .route("/get_status", web::get().to(get_status))
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}
