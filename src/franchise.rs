//! The league's franchise history: who is in the league each year, where they play, and when
//! teams are founded, move, rename or fold.
//!
//! Team cities are real, but the nicknames are fictional (the project does not ship real team
//! names). Each "franchise" has a lineage of identities, e.g. a club founded as the "Syracuse
//! Salt Kings" in 1949 that moves to Philadelphia in 1963 and becomes the "Liberty". It's all
//! data, so mods can replace it with a totally different league.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub city: String,
    pub nickname: String,
    pub abbr: String,
    pub lon: f64,
    pub lat: f64,
    /// Market size 0-1 (1 = New York).
    pub market: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Franchise {
    pub key: String,
    /// (first season, identity) in order.
    pub eras: Vec<(i32, Identity)>,
    /// Last season the franchise plays (None = still active).
    pub last_year: Option<i32>,
    /// Marked for years beyond real history: only used when future expansion is allowed.
    #[serde(default)]
    pub speculative: bool,
}

impl Franchise {
    pub fn first_year(&self) -> i32 {
        self.eras[0].0
    }
    pub fn active_in(&self, year: i32) -> bool {
        year >= self.first_year() && self.last_year.map(|l| year <= l).unwrap_or(true)
    }
    pub fn identity_in(&self, year: i32) -> &Identity {
        let mut cur = &self.eras[0].1;
        for (y, id) in &self.eras {
            if *y <= year {
                cur = id;
            }
        }
        cur
    }
}

fn id(city: &str, nick: &str, abbr: &str, lon: f64, lat: f64, market: f64) -> Identity {
    Identity { city: city.into(), nickname: nick.into(), abbr: abbr.into(), lon, lat, market }
}

fn fr(key: &str, eras: Vec<(i32, Identity)>, last: Option<i32>) -> Franchise {
    Franchise { key: key.into(), eras, last_year: last, speculative: false }
}

/// The built-in franchise history, 1946 onward.
pub fn builtin_franchises() -> Vec<Franchise> {
    vec![
        // ---- Original eleven (1946) ----
        fr("bos", vec![(1946, id("Boston", "Harbormen", "BOS", -71.06, 42.36, 0.78))], None),
        fr("nyk", vec![(1946, id("New York", "Skyliners", "NYS", -74.0, 40.71, 1.0))], None),
        fr("gsw", vec![(1946, id("Philadelphia", "Founders", "PHF", -75.17, 39.95, 0.8)), (1962, id("San Francisco", "Foghorns", "SFF", -122.42, 37.77, 0.7)), (1971, id("Oakland", "Foghorns", "OKF", -122.27, 37.8, 0.62))], None),
        fr("tor_h", vec![(1946, id("Toronto", "Huskies", "TOH", -79.38, 43.65, 0.4))], Some(1946)),
        fr("pro", vec![(1946, id("Providence", "Steamers", "PRO", -71.41, 41.82, 0.2))], Some(1947)),
        fr("pit", vec![(1946, id("Pittsburgh", "Ironclads", "PIT", -79.99, 40.44, 0.45))], Some(1946)),
        fr("cle_f", vec![(1946, id("Cleveland", "Foresters", "CLF", -81.69, 41.5, 0.5))], Some(1948)),
        fr("det_f", vec![(1946, id("Detroit", "Falcons", "DEF", -83.05, 42.33, 0.55))], Some(1946)),
        fr("chi_s", vec![(1946, id("Chicago", "Smokestacks", "CHS", -87.63, 41.88, 0.9))], Some(1949)),
        fr("wsh_c", vec![(1946, id("Washington", "Capitals", "WSC", -77.04, 38.9, 0.5))], Some(1950)),
        fr("stl_b", vec![(1946, id("St. Louis", "Bombers", "STB", -90.2, 38.63, 0.4))], Some(1949)),
        fr("bal_b", vec![(1947, id("Baltimore", "Bayhawks", "BAL", -76.61, 39.29, 0.35))], Some(1953)),
        // ---- Merger (1949) ----
        fr("phi", vec![(1949, id("Syracuse", "Salt Kings", "SYR", -76.15, 43.05, 0.15)), (1963, id("Philadelphia", "Liberty", "PHL", -75.17, 39.95, 0.8))], None),
        fr("lal", vec![(1949, id("Minneapolis", "Loons", "MNL", -93.27, 44.98, 0.4)), (1960, id("Los Angeles", "Sunsets", "LAS", -118.24, 34.05, 0.98))], None),
        fr("sac", vec![(1949, id("Rochester", "Flyers", "ROC", -77.61, 43.16, 0.12)), (1957, id("Cincinnati", "Rangers", "CIN", -84.51, 39.1, 0.3)), (1972, id("Kansas City", "Pioneers", "KCP", -94.58, 39.1, 0.3)), (1985, id("Sacramento", "Gold Rush", "SAC", -121.49, 38.58, 0.3))], None),
        fr("det", vec![(1949, id("Fort Wayne", "Hammers", "FTW", -85.14, 41.08, 0.08)), (1957, id("Detroit", "Gears", "DET", -83.05, 42.33, 0.55))], None),
        fr("atl", vec![(1949, id("Tri-Cities", "Rivermen", "TRI", -90.57, 41.52, 0.1)), (1951, id("Milwaukee", "Brewers", "MIB", -87.91, 43.04, 0.3)), (1955, id("St. Louis", "Gateway", "STG", -90.2, 38.63, 0.45)), (1968, id("Atlanta", "Peaches", "ATL", -84.39, 33.75, 0.6))], None),
        fr("ind_o", vec![(1949, id("Indianapolis", "Olympians", "INO", -86.16, 39.77, 0.3))], Some(1952)),
        fr("den_s", vec![(1949, id("Denver", "Smelters", "DNS", -104.99, 39.74, 0.3))], Some(1949)),
        fr("and", vec![(1949, id("Anderson", "Packers", "AND", -85.68, 40.1, 0.03))], Some(1949)),
        fr("wat", vec![(1949, id("Waterloo", "Hawks", "WAT", -92.34, 42.49, 0.03))], Some(1949)),
        fr("she", vec![(1949, id("Sheboygan", "Redskins", "SHE", -87.71, 43.75, 0.02))], Some(1949)),
        // ---- Expansion ----
        fr("was", vec![(1961, id("Chicago", "Packers", "CHP", -87.63, 41.88, 0.9)), (1963, id("Baltimore", "Clippers", "BCL", -76.61, 39.29, 0.35)), (1973, id("Capital", "Monuments", "CAP", -77.04, 38.9, 0.55)), (1997, id("Washington", "Monuments", "WAS", -77.04, 38.9, 0.62))], None),
        fr("chi", vec![(1966, id("Chicago", "Blizzards", "CHI", -87.63, 41.88, 0.9))], None),
        fr("okc", vec![(1967, id("Seattle", "Evergreens", "SEA", -122.33, 47.61, 0.55)), (2008, id("Oklahoma City", "Twisters", "OKC", -97.52, 35.47, 0.18))], None),
        fr("hou", vec![(1967, id("San Diego", "Surf", "SDS", -117.16, 32.72, 0.4)), (1971, id("Houston", "Gushers", "HOU", -95.37, 29.76, 0.7))], None),
        fr("mil", vec![(1968, id("Milwaukee", "Stags", "MIL", -87.91, 43.04, 0.3))], None),
        fr("phx", vec![(1968, id("Phoenix", "Dust Devils", "PHX", -112.07, 33.45, 0.55))], None),
        fr("lac", vec![(1970, id("Buffalo", "Bison", "BUF", -78.88, 42.89, 0.2)), (1978, id("San Diego", "Waves", "SDW", -117.16, 32.72, 0.4)), (1984, id("Los Angeles", "Waves", "LAW", -118.24, 34.05, 0.9))], None),
        fr("cle", vec![(1970, id("Cleveland", "Mariners", "CLE", -81.69, 41.5, 0.45))], None),
        fr("por", vec![(1970, id("Portland", "Loggers", "POR", -122.68, 45.52, 0.4))], None),
        fr("uta", vec![(1974, id("New Orleans", "Jazzmen", "NOJ", -90.07, 29.95, 0.35)), (1979, id("Utah", "Peaks", "UTA", -111.89, 40.76, 0.22))], None),
        fr("den", vec![(1976, id("Denver", "Prospectors", "DEN", -104.99, 39.74, 0.4))], None),
        fr("ind", vec![(1976, id("Indiana", "Racers", "IND", -86.16, 39.77, 0.3))], None),
        fr("bkn", vec![(1976, id("New Jersey", "Marshes", "NJM", -74.17, 40.73, 0.55)), (2012, id("Brooklyn", "Borough", "BKB", -73.94, 40.65, 0.9))], None),
        fr("sas", vec![(1976, id("San Antonio", "Missions", "SAS", -98.49, 29.42, 0.28))], None),
        fr("dal", vec![(1980, id("Dallas", "Wranglers", "DAL", -96.8, 32.78, 0.7))], None),
        fr("nop", vec![(1988, id("Charlotte", "Queens", "CHQ", -80.84, 35.23, 0.3)), (2002, id("New Orleans", "Bayou", "NOB", -90.07, 29.95, 0.3))], None),
        fr("mia", vec![(1988, id("Miami", "Flamingos", "MIA", -80.19, 25.76, 0.65))], None),
        fr("orl", vec![(1989, id("Orlando", "Sunbirds", "ORL", -81.38, 28.54, 0.45))], None),
        fr("min", vec![(1989, id("Minneapolis", "Frost", "MIN", -93.27, 44.98, 0.4))], None),
        fr("tor", vec![(1995, id("Toronto", "Northmen", "TOR", -79.38, 43.65, 0.65))], None),
        fr("mem", vec![(1995, id("Vancouver", "Orcas", "VAN", -123.12, 49.28, 0.3)), (2001, id("Memphis", "Blues", "MEM", -90.05, 35.15, 0.2))], None),
        fr("cha", vec![(2004, id("Charlotte", "Stingers", "CHA", -80.84, 35.23, 0.3))], None),
        // ---- Speculative future expansion ----
        Franchise { speculative: true, ..fr("sea2", vec![(2028, id("Seattle", "Rain", "SEA", -122.33, 47.61, 0.55))], None) },
        Franchise { speculative: true, ..fr("lv", vec![(2028, id("Las Vegas", "Gamblers", "LVG", -115.14, 36.17, 0.45))], None) },
    ]
}

/// Franchises active in a given season.
pub fn active_in(franchises: &[Franchise], year: i32, include_speculative: bool) -> Vec<usize> {
    franchises.iter().enumerate().filter(|(_, f)| f.active_in(year) && (include_speculative || !f.speculative)).map(|(i, _)| i).collect()
}

/// Great-circle distance in miles.
pub fn miles_between(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let r = 3958.8;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dphi = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dphi / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn league_size_follows_history() {
        let f = builtin_franchises();
        assert_eq!(active_in(&f, 1946, false).len(), 11);
        let n1976 = active_in(&f, 1976, false).len();
        let n1996 = active_in(&f, 1996, false).len();
        let n2010 = active_in(&f, 2010, false).len();
        assert!(n1976 >= 20 && n1976 <= 24, "1976 had {n1976}");
        assert_eq!(n1996, 29);
        assert_eq!(n2010, 30);
        assert_eq!(active_in(&f, 2030, true).len(), 32);
    }

    #[test]
    fn identities_change_over_time() {
        let f = builtin_franchises();
        let phi = f.iter().find(|x| x.key == "phi").unwrap();
        assert_eq!(phi.identity_in(1950).city, "Syracuse");
        assert_eq!(phi.identity_in(1970).city, "Philadelphia");
        assert!(miles_between(-74.0, 40.7, -118.2, 34.0) > 2400.0);
    }
}
