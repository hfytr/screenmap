#![feature(path_file_prefix)]
use anyhow::{Error, Result};
use sqlx::PgPool;
use std::fmt::Display;
use std::fs::{File, canonicalize};
use std::path::PathBuf;
use std::{env, process::exit};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum RecordType {
    // rank by MAX - MIN
    SMALLINT = 0,
    INT = 1,
    BIGINT = 2,
    REAL = 3,
    DOUBLE = 4,
    TEXT = 5,
}

impl Display for RecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordType::SMALLINT => f.write_str("SMALLINT"),
            RecordType::INT => f.write_str("INT"),
            RecordType::BIGINT => f.write_str("BIGINT"),
            RecordType::REAL => f.write_str("REAL"),
            RecordType::DOUBLE => f.write_str("DOUBLE"),
            RecordType::TEXT => f.write_str("TEXT"),
        }
    }
}

struct Args {
    pub fpath: PathBuf,
    pub force: bool,
}

impl Args {
    fn new(args_raw: Vec<String>) -> Args {
        let mut args = Args {
            fpath: PathBuf::new(),
            force: false,
        };
        let mut fpath = None;
        for arg in args_raw.iter().skip(1) {
            if arg == "-f" {
                if args.force {
                    eprintln!("ERROR: Only pass -f once.");
                } else {
                    args.force = true;
                }
            } else if fpath.is_some() {
                eprintln!("ERROR: Extraneous argument {:?} found.", arg);
                exit(1);
            } else {
                let err_msg = "ERROR: Failed to canonicalize path.";
                fpath = Some(canonicalize(PathBuf::from(&arg)).expect(err_msg));
            }
        }
        if args_raw.len() < 2 {
            eprintln!("ERROR: Too few arguments supplied. Supply the input file as argument 1.");
            exit(1);
        } else if args_raw.len() > 3 {
            eprintln!(
                "ERROR: Too many arguments supplied. Only supply the input file as argument 1."
            );
            exit(1);
        }
        args.fpath = fpath.expect("ERROR: Supply a csv file to add to the db.");
        args
    }
}

struct CSVProcessor {
    table_name: String,
    header_types: Vec<RecordType>,
    columns: Vec<String>,
    fpath: PathBuf,
}

trait ValidateForSQL {
    fn validate(&self) -> String;
}
impl ValidateForSQL for &str {
    fn validate(&self) -> String {
        if self == &"id" {
            eprintln!("ERROR: use of reserved word 'id' in csv.")
        }
        self.replace(" ", "_")
            .replace("-", "_")
            .replace("(", "_")
            .replace(")", "_")
            .replace(".", "_")
    }
}

impl CSVProcessor {
    fn new(fpath: PathBuf) -> Result<Self> {
        let err_msg = format!("Failed to read file {:?} passed on command line.", fpath);
        let mut reader = csv::ReaderBuilder::new().from_reader(File::open(&fpath).expect(&err_msg));
        let err_msg = "Failed to parse input headers";
        let headers = reader
            .headers()
            .expect(err_msg)
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let num_headers = headers.len();
        let mut header_types = vec![None; num_headers];
        for (i, maybe_row) in reader.records().enumerate() {
            if let Err(e) = maybe_row {
                eprintln!("WARNING: Error parsing csv on line {}: {:?}", i + 1, e);
            } else if let Ok(row) = maybe_row {
                for (maybe_header_type, item) in header_types.iter_mut().zip(row.iter()) {
                    let smallest_type = if let Ok(x) = str::parse::<i64>(item) {
                        if x < i16::MAX.into() && x > i16::MIN.into() {
                            RecordType::SMALLINT
                        } else if x < i32::MAX.into() && x > i32::MIN.into() {
                            RecordType::INT
                        } else {
                            RecordType::BIGINT
                        }
                    } else if let Ok(x) = str::parse::<f64>(item) {
                        if x < f32::MAX.into() && x > f32::MIN.into() {
                            RecordType::REAL
                        } else {
                            RecordType::DOUBLE
                        }
                    } else {
                        RecordType::TEXT
                    };
                    if let Some(header_type) = maybe_header_type && smallest_type > *header_type {
                        *header_type = smallest_type
                    } else {
                        *maybe_header_type = Some(smallest_type);
                    }
                }
            }
        }

        let columns: Vec<_> = headers.iter().map(|s| s.as_str().validate()).collect();
        for c in columns.iter() {
            if c.chars().next().expect("ERROR: Empty column name.").is_digit(10) {
                let err_msg = format!("ERROR: Invalid column name starting with digit: {}.", c);
                eprintln!("{}", err_msg);
            }
        }
        let header_types: Vec<_> = header_types.into_iter().map(Option::unwrap).collect();
        println!("CSV header -> SQL column: Detected Type");
        headers
            .iter()
            .zip(columns.iter())
            .zip(header_types.iter())
            .for_each(|((header, column), header_type)| {
                println!("{} -> {}: {}", header, column, header_type)
            });

        let table_name = fpath
            .file_prefix()
            .ok_or(Error::msg("ERROR: Failed getting file prefix"))?
            .to_str()
            .ok_or(Error::msg("ERROR: Failed converting OsStr to string"))?
            .validate()
            .to_ascii_lowercase();
        Ok(Self {
            table_name,
            columns,
            header_types,
            fpath,
        })
    }

    async fn write_db(&self, db_url: &str, force: bool) -> Result<()> {
        let pool = PgPool::connect(db_url).await?;
        println!("{}", self.table_name);
        let table_exists = sqlx::query!(
            "SELECT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = $1)",
            self.table_name
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .exists
        .unwrap();
        println!("{}", table_exists);
        if table_exists {
            if force {
                let drop_table_q = format!("DROP TABLE {}", self.table_name);
                println!("INFO: Running SQL query \"{}\";", drop_table_q);
                sqlx::query(&drop_table_q).execute(&pool).await?;
            } else {
                eprintln!(
                    "ERROR: Table with name (derived from input file) {:?} already exists. Pass -f to delete existing table (probably a bad idea)",
                    self.table_name
                );
            }
        }

        let mut create_tbl_q = self.columns.iter().zip(self.header_types.iter()).fold(
            format!("CREATE TABLE {}(", self.table_name),
            |acc, (header, header_type)| format!("{}{} {},", acc, header, header_type),
        );
        create_tbl_q.pop();
        create_tbl_q.push(')');
        println!("INFO: Running SQL query \"{}\";", create_tbl_q);
        sqlx::query(&create_tbl_q).execute(&pool).await?;

        let mut copy_csv_q = self
            .columns
            .iter()
            .fold(format!("COPY {}(", self.table_name), |acc, header| {
                format!("{}{},", acc, header)
            });
        copy_csv_q.pop();
        let fpath_str = self.fpath.to_str().unwrap();
        copy_csv_q = format!(
            "{}) FROM '{}' DELIMITER ',' CSV HEADER",
            copy_csv_q, fpath_str
        );
        println!("INFO: Running SQL query \"{}\";", copy_csv_q);
        sqlx::query(&copy_csv_q).execute(&pool).await?;

        let add_id_q = format!(
            "ALTER TABLE {} ADD COLUMN id SERIAL PRIMARY KEY",
            self.table_name
        );
        println!("INFO: Running SQL query \"{}\";", add_id_q);
        sqlx::query(&add_id_q).execute(&pool).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let args = Args::new(env::args().collect());
    let err_msg = "ERROR: Set the DATABASE_URL environment variable";
    let db_url = env::var("DATABASE_URL").expect(err_msg);
    let processor = CSVProcessor::new(args.fpath).unwrap();
    processor.write_db(&db_url, args.force).await.unwrap();
}
