use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::{self, Write};
use std::process::{self, Command, Stdio};

use chrono::Utc;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <ics_file> <neomutt_config>", args[0]);
        process::exit(1);
    }
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    let ics_file = &args[1];
    let ics_content = read_ics(ics_file);

    let neomutt_config_file = &args[2];

    let mut uid = String::new();
    let mut organizer = String::new();
    let mut new_sequence: u32 = 0;
    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    let mut recipient = String::new();
    let mut event_title = String::new();


    for line in ics_content {
        if line.starts_with("UID:") {
            uid = line.trim_start_matches("UID:").to_string();
        } else if line.starts_with("ORGANIZER") {
            organizer = line.to_string();
            if line.contains("mailto:") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 1 {
                    recipient = parts.last().expect("Organizer does not contain email").trim().to_string();
                }
            }
        } else if line.starts_with("SEQUENCE:") {
            let sequence = line.trim_start_matches("SEQUENCE:").to_string();
            new_sequence = sequence.parse().unwrap_or(0) + 1;
        } else if line.starts_with("SUMMARY:") {
            event_title = line.trim_start_matches("SUMMARY:").to_string();
        }
    }

    let participation_status = query_user_participation(&ics_file, &mut uid);

    let subject = match participation_status.as_str() {
        "ACCEPTED" => format!("Accepted: {}", event_title),
        "TENTATIVE" => format!("Tentatively Accepted: {}", event_title),
        "DECLINED" => format!("Declined: {}", event_title),
        _ => "Calendar Response".to_string(),
    };

    let sender = get_sender_address();
    let mime_part = format!(
        "BEGIN:VCALENDAR\n\
        PRODID:RSVP-v{version}\n\
        VERSION:2.0\n\
        METHOD:REPLY\n\
        BEGIN:VEVENT\n\
        UID:{uid}\n\
        ATTENDEE;PARTSTAT={participation_status};RSVP=FALSE:mailto:{sender}\n\
        {organizer}\n\
        DTSTAMP:{dtstamp}\n\
        SEQUENCE:{new_sequence}\n\
        END:VEVENT\n\
        END:VCALENDAR"
    );

    send_email_with_neomutt(&mime_part, &recipient, &subject, &neomutt_config_file);
}

fn query_user_participation(ics_file: &&String, uid: &mut String) -> String {
    println!("How to respond to this invitation? [a]ccept / [t]entative / [d]ecline?");

    let mut response_type = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut response_type).expect("Failed to read input");

    let partstat = match response_type.trim().to_uppercase().as_str() {
        "A" => {
            import_to_khal(&ics_file);
            "ACCEPTED"
        }
        "T" => {
            import_to_khal(&ics_file);
            "TENTATIVE"
        }
        "D" => {
            remove_from_khal(&uid);
            "DECLINED"
        }
        _ => {
            eprintln!("Invalid response type. Please enter [a], [t] or [d].");
            process::exit(1);
        }
    };
    partstat.to_string()
}

fn get_sender_address() -> String {
    let output = Command::new("neomutt")
        .arg("-Q")
        .arg("from")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let from = stdout.split(" ").last().expect("No email sender configured");
    from.replace("\"", "").trim().to_string()
}

fn import_to_khal(ics_file: &str) {
    let mut child = Command::new("khal")
        .arg("import")
        .arg(ics_file)
        .spawn()
        .expect("Failed to execute khal import");

    let status = child.wait().expect("Failed to wait on khal edit");

    if !status.success() {
        eprintln!("Failed to import event to khal:");
        process::exit(1);
    }
}

fn remove_from_khal(uid: &str) {
    let mut child = Command::new("khal")
        .arg("edit")
        .arg("--show-past")
        .arg(uid)
        .spawn()
        .expect("Failed to execute khal edit");

    let status = child.wait().expect("Failed to wait on khal edit");

    if !status.success() {
        eprintln!("Failed to remove event from khal");
        process::exit(1);
    }
}

fn send_email_with_neomutt(mime_part: &str, recipient: &str, subject: &str, neomutt_config_file: &str) {
    let path_to_ics_attachment = "/tmp/response.ics";
    let mut ics_attachment = File::create(path_to_ics_attachment).expect("Failed to create temporary file");// NamedTempFile::new().expect("Failed to create temporary file");
    ics_attachment.write_all(mime_part.as_bytes()).expect("Failed to write MIME part to file");

    let status = Command::new("neomutt")
        .arg("-F")
        .arg(neomutt_config_file)
        .arg("-s")
        .arg(subject)
        .arg("-a")
        .arg(path_to_ics_attachment)
        .arg("--")
        .arg(recipient)
        .stdin(Stdio::null())
        .status()
        .expect("Failed to execute Neomutt");

    if !status.success() {
        eprintln!("Failed to send email with Neomutt: {status}");
        process::exit(1);
    }
}

fn read_ics(file_path: &str) -> Vec<String> {
    println!("Reading ics file {file_path}");
    let file = File::open(file_path).expect("Failed to read .ics file");
    let reader = BufReader::new(file);

    let mut final_lines = Vec::new();
    let mut temporary_line = String::new();

    for unprocessed_line_result in reader.lines() {
        let unprocessed_line = unprocessed_line_result.expect("Failed to read line");

        if unprocessed_line.starts_with(char::is_uppercase) {
            if !temporary_line.is_empty() {
                final_lines.push(temporary_line.clone());
                temporary_line.clear();
            }
            temporary_line.push_str(&unprocessed_line);
        } else {
            temporary_line.push_str(&unprocessed_line.trim());
        }
    }

    if !temporary_line.is_empty() {
        final_lines.push(temporary_line);
    }

    final_lines
}