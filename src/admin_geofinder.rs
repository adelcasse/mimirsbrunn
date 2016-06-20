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
use mimir::Admin;
use geo::contains::Contains;
use geo;

pub struct AdminGeoFinder {
    //quadtree: ntree::NTree<QuadTreeRegion, Admin>,
    admins: Vec<Admin>
}

impl AdminGeoFinder {
    pub fn new() -> AdminGeoFinder {
        AdminGeoFinder {
            admins: vec![]
        }
    }

    pub fn add_admin(&mut self, admin: Admin) {
        self.admins.push(admin);
    }

    /// Get all Admins overlaping the coordinate
    pub fn get_admins_for_coord(&self, coord: &geo::Coordinate) -> Vec<&Admin> {
        self.admins.iter().filter(|a| {
                a.boundary.as_ref().map_or(false, |b| {
                    b.contains(&geo::Point(coord.clone()))
                })
            })
            .collect()
    }

    pub fn get_admins_for_street(&self) -> Vec<&Admin> {
        panic!("get_admin_for_street");
    }
}