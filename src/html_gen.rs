use crate::{DataFile, LineStatus};
use std::fmt::Write;
use std::io::BufRead;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
};

fn gen_line(writer: &mut String, index: usize, line: &str, status: LineStatus) {
    let background = match status {
        LineStatus::NotInBinary => "has-background-light",
        LineStatus::NoPanic => "has-background-success-light",
        LineStatus::Panic => "has-background-danger-light",
    };

    write!(
        writer,
        r##"
<div class="columns p-0 m-0" role="row">
    <div class="column is-1 is-narrow p-0 has-text-centered" id="{0}" role="cell">
        <a href="#{0}">{0}</a>
    </div>
    <div class="column {1} p-0" role="cell">
        <pre class="{1} py-0 px-2">{2}</pre>
    </div>
</div>"##,
        index, background, line
    )
    .unwrap();
}

fn gen_file(writer: &mut String, input_file: &str, lines_status: &[LineStatus]) {
    let no_of_panic_lines = lines_status
        .iter()
        .filter(|&&x| x == LineStatus::Panic)
        .count();
    let clean_lines = lines_status.len() - no_of_panic_lines;
    let percent = clean_lines as f32 / lines_status.len() as f32 * 100.0;
    let no_of_lines = lines_status.len();
    let line_background = if percent >= 80.0 {
        "has-text-success"
    } else if percent >= 50.0 {
        "has-text-warning"
    } else {
        "has-text-danger"
    };

    write!(
        writer,
        r#"
<!DOCTYPE html>
<html lang="en-us">

<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{input_file}</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.1/css/bulma.min.css">
</head>

<body>
    <div class="container">
    <nav class="level">
        <div class="level-item has-text-centered">
            <div>
                <p class="heading">Lines</p>
                <p class="title {line_background}">
                    {percent:.2}% ({clean_lines} / {no_of_lines})
                </p>
            </div>
        </div>
    </nav>
        "#,
    )
    .unwrap();

    let mut input_file = BufReader::new(File::open(input_file).unwrap());
    let mut line = String::with_capacity(128);
    let mut line_index = 1;
    loop {
        line.clear();
        if input_file.read_line(&mut line).unwrap() == 0 {
            break;
        }
        line.pop();
        let status = if line_index < lines_status.len() {
            lines_status[line_index]
        } else {
            LineStatus::NotInBinary
        };
        gen_line(writer, line_index, &line, status);

        line_index += 1;
    }

    *writer += "
</body>
</html>
";
}

pub(super) fn gen(output_folder: &str, files: &HashMap<&str, DataFile>) {
    let mut file_path = output_folder.to_string();
    file_path.push('/');
    let file_path_original_size = file_path.len();
    let mut tmp_data = String::with_capacity(4096);

    for (name, data) in files {
        if !name.ends_with("src\\main.rs") {
            continue;
        }

        tmp_data.clear();
        file_path.truncate(file_path_original_size);
        let out_name = name.replace(|x| ['\\', '.'].contains(&x), "_");
        file_path += &out_name;
        file_path += ".html";

        gen_file(&mut tmp_data, name, &data.lines);

        fs::write(&file_path, &tmp_data).unwrap();
    }
}
