use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> WebDriverResult<()> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await?;

    let mut majors: Vec<Major> = vec![];

    // Navigate to https://wikipedia.org.
    driver
        .goto("https://apply.kku.ac.th/programsearch67/programlist.php")
        .await?;
    let faculty_name_dropdown = driver.find(By::Id("facultyname")).await?;

    let faculty_options = faculty_name_dropdown.find_all(By::Tag("option")).await?;

    let options_count = faculty_options.len();

    for i in 1..options_count {
        let option = &faculty_options.get(i);
        if let Some(option) = option {
            option.click().await?;
            let title = option.text().await?;
            majors.append(&mut get_faculty_data(&driver, &title).await?);
        }
    }

    // Always explicitly close the browser.
    driver.quit().await?;

    write_data_as_json::<Vec<Major>>(&majors).await;

    Ok(())
}

#[derive(Debug, Default, Serialize)]
struct Major {
    id: String,
    faculty: String,
    name: String,
    student_in_regular: i16,
    student_in_special: i16,
    scores: HashMap<String, i8>,
}

async fn get_faculty_data(driver: &WebDriver, name: &str) -> WebDriverResult<Vec<Major>> {
    // XPath to score table
    let table_elm = driver.query(By::XPath(
        "/html/body/div[2]/div[3]/div/div/div/div[1]/div[2]/table",
    ));

    driver
        .set_implicit_wait_timeout(std::time::Duration::from_secs(3))
        .await?;

    let mut result: Vec<Major> = vec![];

    if let Some(table) = table_elm.first_opt().await? {
        let tr = &table.query(By::Tag("tr")).all().await?;
        let mut subject_names: Vec<String> = vec![];

        // this has nothing to do with the score table
        get_major_names(tr, &mut subject_names).await?;

        let tbody = table.query(By::Tag("tbody")).first().await?;
        let major_rows = tbody.query(By::Tag("tr")).all().await?;
        for major_row in major_rows {
            let major_td = major_row.query(By::Tag("td")).all().await?;
            let major = get_major_data(major_td, name, &subject_names).await?;
            result.push(major);
        }
    }
    Ok(result)
}

async fn get_major_data(
    major_data: Vec<WebElement>,
    faculty: &str,
    subject_names: &[String],
) -> WebDriverResult<Major> {
    let mut major = Major::default();
    for data in 0..major_data.len() {
        let text = major_data.get(data).unwrap().text().await?;
        match data {
            0 => major.id = text,
            1 => major.name = text.replace('*', ""),
            2 => major.student_in_regular = text.parse().unwrap_or(0),
            3 => major.student_in_special = text.parse().unwrap_or(0),
            _ => {
                let subject = subject_names.get(data - 4);

                if let Some(subject) = subject {
                    let score = text.parse().unwrap_or(0);
                    if score <= 0 {
                        continue;
                    }
                    major.scores.insert(subject.to_string(), score);
                }
            }
        }
    }

    major.faculty = faculty.to_string();
    Ok(major)
}

async fn get_major_names(
    tr: &[WebElement],
    subject_names: &mut Vec<String>,
) -> WebDriverResult<()> {
    if let Some(tr) = tr.get(1) {
        let th_vec = tr.query(By::Tag("th")).all().await?.to_vec();
        // exclude regular and special student th tag
        for th in th_vec.iter().skip(2) {
            let subject = th.text().await?.split_whitespace().collect::<Vec<&str>>()[0].to_string();
            subject_names.push(subject);
        }
    }
    Ok(())
}

async fn write_data_as_json<T>(data: &T)
where
    T: Serialize,
{
    let file_path = "netsat_data.json";

    let json = serde_json::to_string_pretty(data).expect("Unable to serialize data");
    let mut file = File::create(file_path).expect("Unable to create file");

    file.write_all(json.as_bytes())
        .expect("Unable to write data to file");
}
