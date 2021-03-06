// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io

extern crate csv;
extern crate geo;
#[macro_use]
extern crate log;
extern crate mimir;
extern crate mimirsbrunn;
#[macro_use]
extern crate serde_derive;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::path::Path;
use mimir::rubber::Rubber;
use mimir::objects::{Addr, Admin, MimirObject};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use std::fs;
use std::rc::Rc;
use std::collections::BTreeMap;
use structopt::StructOpt;

type AdminFromInsee = BTreeMap<String, Rc<Admin>>;

#[derive(Serialize, Deserialize)]
pub struct Bano {
    pub id: String,
    pub nb: String,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub src: String,
    pub lat: f64,
    pub lon: f64,
}

impl Bano {
    pub fn insee(&self) -> &str {
        assert!(self.id.len() >= 5);
        self.id[..5].trim_left_matches('0')
    }
    pub fn fantoir(&self) -> &str {
        assert!(self.id.len() >= 10);
        &self.id[..10]
    }
    pub fn into_addr(
        self,
        admins_from_insee: &AdminFromInsee,
        admins_geofinder: &AdminGeoFinder,
    ) -> mimir::Addr {
        let street_name = format!("{} ({})", self.street, self.city);
        let addr_name = format!("{} {}", self.nb, self.street);
        let addr_label = format!("{} ({})", addr_name, self.city);
        let street_id = format!("street:{}", self.fantoir().to_string());
        let mut admins = admins_geofinder.get(&geo::Coordinate {
            x: self.lat,
            y: self.lon,
        });

        // If we have an admin corresponding to the INSEE, we know
        // that's the good one, thus we remove all the admins of its
        // level found by the geofinder, and add our admin.
        if let Some(admin) = admins_from_insee.get(self.insee()) {
            admins.retain(|a| a.level != admin.level);
            admins.push(admin.clone());
        }

        let weight = admins
            .iter()
            .find(|a| a.level == 8)
            .map_or(0., |a| a.weight.get());

        let street = mimir::Street {
            id: street_id,
            street_name: self.street,
            label: street_name.to_string(),
            administrative_regions: admins,
            weight: weight,
            zip_codes: vec![self.zip.clone()],
            coord: mimir::Coord::new(self.lat, self.lon),
        };
        mimir::Addr {
            id: format!("addr:{};{}", self.lon, self.lat),
            house_number: self.nb,
            street: street,
            label: addr_label,
            coord: mimir::Coord::new(self.lat, self.lon),
            weight: weight,
            zip_codes: vec![self.zip.clone()],
        }
    }
}

fn index_bano<I>(cnx_string: &str, dataset: &str, files: I)
where
    I: Iterator<Item = std::path::PathBuf>,
{
    let mut rubber = Rubber::new(cnx_string);
    rubber.initialize_templates().unwrap();

    let admins = rubber
        .get_admins_from_dataset(dataset)
        .unwrap_or_else(|err| {
            info!(
                "Administratives regions not found in es db for dataset {}. (error: {})",
                dataset, err
            );
            vec![]
        });
    let admins_geofinder = admins.iter().cloned().collect();
    let admins_by_insee = admins
        .into_iter()
        .filter(|a| !a.insee.is_empty())
        .map(|mut a| {
            a.boundary = None; // to save some space we remove the admin boundary
            (a.insee.clone(), Rc::new(a))
        })
        .collect();

    let addr_index = rubber.make_index(Addr::doc_type(), dataset).unwrap();
    info!("Add data in elasticsearch db.");
    for f in files {
        info!("importing {:?}...", &f);
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(&f)
            .unwrap();

        let iter = rdr.deserialize().map(|r| {
            let b: Bano = r.unwrap();
            b.into_addr(&admins_by_insee, &admins_geofinder)
        });
        match rubber.bulk_index(&addr_index, iter) {
            Err(e) => panic!("failed to bulk insert file {:?} because: {}", &f, e),
            Ok(nb) => info!("importing {:?}: {} addresses added.", &f, nb),
        }
    }
    rubber
        .publish_index(Addr::doc_type(), dataset, addr_index, Addr::is_geo_data())
        .unwrap();
}

#[derive(StructOpt, Debug)]
struct Args {
    /// Bano files. Can be either a directory or a file.
    #[structopt(short = "i", long = "input")]
    input: String,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long = "connection-string",
                default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
}

fn main() {
    mimir::logger_init().unwrap();
    info!("importing bano into Mimir");

    let args = Args::from_args();

    let file_path = Path::new(&args.input);
    if file_path.is_dir() {
        let paths: std::fs::ReadDir = fs::read_dir(&args.input).unwrap();
        index_bano(
            &args.connection_string,
            &args.dataset,
            paths.map(|p| p.unwrap().path()),
        );
    } else {
        index_bano(
            &args.connection_string,
            &args.dataset,
            std::iter::once(std::path::PathBuf::from(&args.input)),
        );
    }
}
