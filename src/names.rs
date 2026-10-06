//! Names, countries, hometowns, schools and clubs.
//!
//! All names are fictional combinations. Everything here is data and can be replaced by a mod
//! (for example, a pack that adds a whole new country with its own name pool).

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamePool {
    pub id: String,
    pub first: Vec<String>,
    pub last: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Country {
    /// Short code ("USA", "SRB").
    pub code: String,
    pub name: String,
    /// Which name pool this country draws from.
    pub pool: String,
    /// Relative share of the international talent supply by year: (year, weight).
    pub weight: Vec<(i32, f64)>,
    pub towns: Vec<String>,
    /// Average height offset in inches vs. the US (Balkan players run tall, etc.).
    pub height_bonus: f64,
    /// Typical overseas league name.
    pub league: String,
}

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(|x| x.replace('_', " ")).collect()
}

fn pool(id: &str, first: &str, last: &str) -> NamePool {
    NamePool { id: id.into(), first: words(first), last: words(last) }
}

pub fn builtin_pools() -> Vec<NamePool> {
    vec![
        pool("us",
            "James Marcus Darnell Terrence Andre Malik Jamal Isaiah Devin Cameron Brandon Tyrone Jordan Elijah Xavier Kendrick Dominic Rashad Quinton DeShawn Trevor Caleb Garrett Hunter Mason Logan Tyler Austin Brett Wesley Calvin Reggie Walter Earl Lamar Donte Jerome Corey Darius Lonnie Roscoe Marvin Clarence Virgil Floyd Chester Nathaniel Otis Alonzo Bernard Gordon Harlan Wendell Julius Booker Leroy Sherman Teddy Randall Curtis Ronnie Kareem Troy Rodney Vince Spencer Preston Gavin Landon Tristan Zion Jaylen Trey Deandre Kobe Amari Cade Ezra",
            "Johnson Williams Carter Brooks Washington Jefferson Mitchell Harris Coleman Reed Bryant Hayes Foster Simmons Powell Henderson Barnes Ross Patterson Hughes Flowers Jenkins Perry Butler Russell Griffin Hunter Gibson Ellis Stevens Murray Ford Graham Wallace Weaver Fields Bishop Holloway Crawford Sullivan Dawson Pierce Banks Lawson Webb Burke Sutton Booker Maddox Whitfield Rowe Calloway Haynes Pruitt Stokes Tillman Vance Wilder Yates Zimmerman Abbott Beckett Chambers Dunn Eaton Fletcher Gaines Hardin Irving Kendall Lowery Monroe Nash Oakes Prescott Quarles Rawls Spencer Thornton Upshaw Vaughn Whitaker Boyd Cannon Dillard Emerson Franklin Goode Hollis Ingram Jordan Knight Lambert Mercer Norris Odom Parrish Rhodes Sampson Tate Underwood Vinson Walton"),
        pool("balkan",
            "Dragan Nikola Marko Luka Stefan Milos Vlade Aleksandar Bogdan Dejan Goran Ivan Zoran Predrag Toni Drazen Dino Miro Danilo Vasilije Nemanja Filip Mario Tomislav Zarko Darko Andrej Matej Jure Goran Radoslav",
            "Petrovic Jovanovic Kovacevic Markovic Djordjevic Radovic Stojanovic Ilic Vukovic Milosevic Savic Kukoc Divac Bogdanovic Jokic Dragic Nedovic Perasovic Pavlovic Vesely Zupan Novak Horvat Babic Knezevic Maric Tadic Matic Rakocevic Spasojevic Kostic Obradovic Stanic Simic Lazarevic Mijatovic Cosic Bjelica Jaric Gavrilovic"),
        pool("baltic",
            "Arvydas Sarunas Rimas Jonas Donatas Linas Mantas Tomas Martins Andris Janis Kristaps Valdas Egidijus Gintaras Ramunas Mindaugas Tadas Deividas Edgaras",
            "Kazlauskas Jasikevicius Petraitis Valanciunas Sabonis Marciulionis Bendzius Porzingis Bertans Kalnietis Macijauskas Motiejunas Ulanovas Gudaitis Berzins Strelnieks Sakalauskas Butkus Kuzminskas Jankunas Zukauskas Lekavicius"),
        pool("iberian",
            "Pau Marc Juan Carlos Sergio Rudy Jose Alberto Ricky Fernando Diego Pablo Luis Manuel Facundo Andres Leandro Marcelo Tiago Rafael Mateo Nicolas Gonzalo Felipe Santiago Joaquin Emanuel Guillermo Hugo Bruno",
            "Garcia Navarro Rodriguez Fernandez Lopez Sanchez Gasol Calderon Rubio Ibaka Scola Ginobili Nocioni Delfino Splitter Varejao Barbosa Huertas Machado Ribeiro Pereira Santos Costa Oliveira Almeida Teixeira Domingo Salazar Vidal Montero Ortega Castillo Reyes Mendoza Cruz Bautista Aguilar"),
        pool("french",
            "Tony Boris Nicolas Joakim Rudy Evan Antoine Thomas Vincent Mickael Florent Yannick Remi Mathias Killian Theo Hugo Louis Maxime Pierre Axel Bastien Clement Damien Gaetan",
            "Parker Diaw Batum Noah Gobert Fournier Diot Lauvergne Ntilikina Batum Heurtel Lessort Hayes Dubois Moreau Laurent Bernard Girard Faure Roux Blanc Perrin Morel Renaud Fontaine Chevalier Lefevre Barbier Colin Muller Michel Leclerc Clement Rey Vidal Carpentier Marechal Lacroix Brunet"),
        pool("german",
            "Dirk Detlef Henning Tibor Dennis Maxi Daniel Isaac Johannes Moritz Franz Paul Lukas Kai Jan Leon Jonas Niklas Fabian Sebastian Tim Nils Robin",
            "Nowitzki Schrempf Harnisch Schroder Kleber Theis Zipser Voigtmann Wagner Bonga Thiemann Hartenstein Mueller Weber Fischer Becker Schulz Hoffmann Koch Richter Klein Wolf Neumann Schwarz Krause Braun Zimmermann Hartmann Lange Werner Krueger Vogel Brandt Haas"),
        pool("greek",
            "Giannis Nick Kostas Vassilis Theo Thanasis Georgios Dimitris Panagiotis Evangelos Ioannis Michalis Sofoklis Nikos Kyriakos Manolis Spiros Andreas Christos Lazaros",
            "Antetokounmpo Galis Papanikolaou Spanoulis Diamantidis Zisis Fotsis Printezis Sloukas Calathes Mitoglou Papagiannis Vezenkov Larentzakis Koufos Karamanolis Kalaitzakis Katsikaris Bourousis Perperoglou Dorsey Papadopoulos Nikolaidis Georgiou Alexiou"),
        pool("turkish",
            "Hedo Mehmet Ersan Omer Enes Furkan Cedi Kerem Semih Emir Burak Berk Can Yigit Ibrahim Baris Metin Mert Onur Tolga",
            "Turkoglu Okur Ilyasova Asik Kanter Korkmaz Osman Gonlum Erden Preldzic Aldemir Dogus Ozmizrak Savas Guler Yilmaz Kaya Demir Celik Sahin Arslan Aydin Ozturk Polat Kurt Yildiz Aksoy Bozkurt"),
        pool("italian",
            "Marco Andrea Danilo Luigi Gigi Nicolo Alessandro Davide Stefano Giorgio Matteo Riccardo Federico Lorenzo Simone Fabio Paolo Giacomo Enrico Antonio",
            "Belinelli Gallinari Datome Bargnani Melli Hackett Gentile Rossi Bianchi Romano Colombo Ricci Marino Greco Bruno Gallo Conti Esposito De_Luca Mancini Costa Giordano Rizzo Lombardi Moretti Barbieri Fontana Santoro Mariani Rinaldi Caruso Ferrara Galli"),
        pool("west_african",
            "Hakeem Dikembe Serge Joel Luol Bismack Pascal Gorgui Ike Festus Chimezie Precious Olumide Mamadi Emeka Samuel Thabo Mouhamed Kenneth Oluwaseun Ousmane Tidjane Cheikh Boubacar Nnamdi Chidi",
            "Olajuwon Mutombo Ibaka Embiid Deng Biyombo Siakam Dieng Nwora Ezeli Metu Achiuwa Adebayo Diallo Okafor Okoro Mbah Ndiaye Sarr Fall Camara Traore Toure Sow Mensah Nwankwo Eze Adeleke Okonkwo Bamba Gueye Konate Ba"),
        pool("chinese",
            "Yao Yi Wang Zhou Sun Ding Zhang Li Chen Liu Wei Hao Jun Ming Tao Lei Feng Bo Kai Rui",
            "Ming Jianlian Zhizhi Qi Yue Peng Wenbo Yuta Xiaolong Zhen Hu Gao Lin Yang Huang Zhao Wu Xu Ma Guo Deng Cao Zhu Han Zeng Peng Su Jiang Ren"),
        pool("japanese",
            "Yuta Rui Yudai Takuma Kosuke Shuhei Keita Ryo Hiroki Daiki Yuki Kenta Naoki Shunsuke Taishi",
            "Tabuse Watanabe Hachimura Baba Nishikawa Kawamura Sato Suzuki Takahashi Tanaka Ito Yamamoto Nakamura Kobayashi Kato Yoshida Yamada Sasaki Yamaguchi Matsumoto Inoue Kimura Hayashi Shimizu"),
        pool("slavic",
            "Andrei Sergei Vladimir Alexei Dmitri Igor Mikhail Yuri Maxim Pavel Viktor Oleg Anton Roman Ivan Artem Timofey Evgeny Kirill Nikita",
            "Kirilenko Belov Sabonis Volkov Mozgov Fridzon Monya Shved Karasev Gladyrev Petrov Ivanov Sokolov Popov Lebedev Kuznetsov Smirnov Morozov Novikov Fedorov Egorov Pavlov Orlov Zaytsev Nikolaev Zhukov Medvedev Tarasov Belyaev"),
        pool("hebrew",
            "Omri Gal Yotam Deni Tal Lior Amit Doron Guy Eyal Yaniv Noam Itay Shay Ori Ran Uri Adi",
            "Casspi Mekel Halperin Avdija Schwartz Levi Cohen Mizrahi Friedman Peretz Biton Katz Avraham Dahan Azoulay Malka Shapiro Ben_David Golan Segal Amar Gabay"),
        pool("nordic",
            "Lauri Erik Jonas Petteri Mikael Sasu Lars Anders Henrik Axel Magnus Joakim Kasper Nils Aleksi Eero Olli Veli Rasmus Mikkel",
            "Markkanen Salin Koponen Lindgren Berg Larsen Andersen Nielsen Hansen Jensen Svensson Pettersson Karlsson Johansson Eriksson Virtanen Korhonen Laine Heikkinen Makela Hamalainen Lahti Salo"),
        pool("caribbean",
            "Al Rafael Tito Jean Carlos Wilfredo Edgardo Alexis Dennis Pedro Winston Orlando Roberto Ramon Kevin Luis Junior Pierre Clyde Delroy",
            "Horford Reyes Santana Arroyo Rivera Medina Tavarez Mercedes Baez Pimentel Peralta Cabral Valdez Jeannot Joseph Louis Bastien Pierre Francois Sands Rolle Wells Munnings Thompson Ferguson Stubbs"),
    ]
}

fn country(code: &str, name: &str, pool: &str, weight: &[(i32, f64)], towns: &str, h: f64, league: &str) -> Country {
    Country { code: code.into(), name: name.into(), pool: pool.into(), weight: weight.to_vec(), towns: words(towns), height_bonus: h, league: league.into() }
}

pub fn builtin_countries() -> Vec<Country> {
    vec![
        country("CAN", "Canada", "us", &[(1946, 3.0), (1990, 3.0), (2005, 5.0), (2020, 9.0)], "Toronto Vancouver Montreal Hamilton Brampton Winnipeg Calgary Halifax", 0.0, "Canadian Elite League"),
        country("SRB", "Serbia", "balkan", &[(1946, 0.2), (1975, 2.0), (1992, 6.0), (2015, 8.0)], "Belgrade Novi_Sad Nis Kragujevac Cacak", 2.2, "Adriatic Premier League"),
        country("CRO", "Croatia", "balkan", &[(1946, 0.1), (1975, 1.5), (1992, 4.0), (2015, 3.0)], "Zagreb Split Zadar Sibenik", 2.0, "Adriatic Premier League"),
        country("SLO", "Slovenia", "balkan", &[(1946, 0.1), (1992, 1.0), (2012, 2.5)], "Ljubljana Maribor Koper", 1.8, "Adriatic Premier League"),
        country("LTU", "Lithuania", "baltic", &[(1946, 0.1), (1988, 2.5), (2000, 3.0), (2020, 2.0)], "Kaunas Vilnius Klaipeda Siauliai", 1.8, "Baltic League"),
        country("LAT", "Latvia", "baltic", &[(1946, 0.05), (1990, 0.5), (2014, 2.0)], "Riga Liepaja Ventspils", 1.8, "Baltic League"),
        country("ESP", "Spain", "iberian", &[(1946, 0.2), (1985, 1.5), (2000, 6.0), (2015, 5.0)], "Barcelona Madrid Valencia Vitoria Malaga", 1.0, "Liga Iberica"),
        country("ARG", "Argentina", "iberian", &[(1946, 0.4), (2000, 2.5), (2012, 1.5)], "Buenos_Aires Bahia_Blanca Cordoba", 0.6, "Liga Iberica"),
        country("BRA", "Brazil", "iberian", &[(1946, 0.3), (1985, 1.5), (2010, 2.0)], "Sao_Paulo Rio Brasilia", 0.8, "Liga Iberica"),
        country("FRA", "France", "french", &[(1946, 0.3), (1990, 1.5), (2005, 5.0), (2020, 9.0)], "Paris Lyon Villeurbanne Strasbourg Le_Mans", 0.8, "French Pro League"),
        country("GER", "Germany", "german", &[(1946, 0.2), (1990, 1.0), (2010, 3.0), (2024, 5.0)], "Berlin Munich Wurzburg Bamberg Ulm", 1.2, "German Pro League"),
        country("GRE", "Greece", "greek", &[(1946, 0.1), (1990, 1.0), (2010, 2.5), (2020, 3.5)], "Athens Thessaloniki Larissa Patras", 0.9, "Hellenic League"),
        country("TUR", "Turkey", "turkish", &[(1946, 0.1), (1995, 0.5), (2005, 2.0), (2015, 2.5)], "Istanbul Ankara Izmir Bursa", 0.8, "Turkish Pro League"),
        country("ITA", "Italy", "italian", &[(1946, 0.3), (1985, 1.5), (2005, 2.0)], "Milan Bologna Rome Treviso Varese", 0.7, "Italian Serie A"),
        country("AUS", "Australia", "us", &[(1946, 0.2), (1990, 1.0), (2010, 4.0), (2022, 6.0)], "Melbourne Sydney Perth Adelaide Brisbane", 1.0, "Southern Pro League"),
        country("NGR", "Nigeria", "west_african", &[(1946, 0.1), (1980, 0.8), (2000, 3.0), (2020, 4.5)], "Lagos Abuja Ibadan Kano Port_Harcourt", 0.8, "West Africa League"),
        country("CMR", "Cameroon", "west_african", &[(1946, 0.05), (1995, 0.5), (2015, 2.0)], "Douala Yaounde Bafoussam", 1.0, "West Africa League"),
        country("SEN", "Senegal", "west_african", &[(1946, 0.05), (1995, 0.7), (2015, 1.5)], "Dakar Thies Saint-Louis", 1.3, "West Africa League"),
        country("COD", "DR Congo", "west_african", &[(1946, 0.05), (1991, 0.5), (2015, 1.2)], "Kinshasa Lubumbashi Goma", 1.5, "West Africa League"),
        country("CHN", "China", "chinese", &[(1946, 0.0), (2000, 0.5), (2005, 0.4), (2020, 0.3)], "Shanghai Beijing Guangzhou Shenzhen Nanjing", 1.4, "Asian Pro League"),
        country("JPN", "Japan", "japanese", &[(1946, 0.0), (2000, 0.1), (2018, 0.5)], "Tokyo Osaka Nagoya Yokohama", 0.0, "Asian Pro League"),
        country("ISR", "Israel", "hebrew", &[(1946, 0.1), (1990, 0.4), (2015, 0.9)], "Tel_Aviv Haifa Jerusalem Eilat", 0.5, "Levant League"),
        country("GBR", "United Kingdom", "us", &[(1946, 0.3), (1995, 0.9), (2018, 1.5)], "London Manchester Birmingham Glasgow Leeds", 0.2, "British Pro League"),
        country("RUS", "Russia / USSR", "slavic", &[(1946, 0.1), (1988, 1.5), (2000, 1.5), (2015, 0.8)], "Moscow St_Petersburg Kazan Samara Krasnodar", 1.3, "Eastern Union League"),
        country("UKR", "Ukraine", "slavic", &[(1946, 0.05), (1992, 0.5), (2010, 1.0), (2020, 1.0)], "Kyiv Kharkiv Dnipro Odesa", 1.3, "Eastern Union League"),
        country("FIN", "Finland", "nordic", &[(1946, 0.0), (1995, 0.2), (2015, 1.0)], "Helsinki Tampere Oulu", 1.2, "Nordic League"),
        country("SWE", "Sweden", "nordic", &[(1946, 0.05), (1995, 0.3), (2015, 0.6)], "Stockholm Gothenburg Malmo", 1.0, "Nordic League"),
        country("DOM", "Dominican Republic", "caribbean", &[(1946, 0.1), (1990, 0.5), (2010, 1.0)], "Santo_Domingo Santiago La_Romana", 0.2, "Caribbean League"),
        country("PUR", "Puerto Rico", "caribbean", &[(1946, 0.3), (1985, 0.8), (2010, 0.4)], "San_Juan Bayamon Ponce", 0.2, "Caribbean League"),
        country("BHS", "Bahamas", "caribbean", &[(1946, 0.1), (1990, 0.3), (2010, 0.7)], "Nassau Freeport", 0.5, "Caribbean League"),
        country("HTI", "Haiti", "caribbean", &[(1946, 0.0), (1995, 0.2), (2015, 0.5)], "Port-au-Prince Cap-Haitien", 0.8, "Caribbean League"),
    ]
}

pub fn us_towns() -> Vec<String> {
    words("New_York Brooklyn The_Bronx Philadelphia Baltimore Washington Richmond Charlotte Atlanta Miami Orlando Houston Dallas San_Antonio New_Orleans Memphis Nashville Louisville Indianapolis Chicago Detroit Cleveland Columbus Cincinnati Pittsburgh Boston Hartford Newark Los_Angeles Oakland Compton Long_Beach San_Diego Phoenix Las_Vegas Seattle Portland Denver Oklahoma_City Kansas_City St._Louis Milwaukee Minneapolis Birmingham Mobile Jackson Shreveport Tulsa Raleigh Greensboro Norfolk Jacksonville Tampa Gary Akron Dayton Buffalo Syracuse Providence Camden Wilmington Fresno Sacramento Salt_Lake_City Albuquerque El_Paso Little_Rock Chattanooga Savannah Macon")
}

pub fn school_words() -> (Vec<String>, Vec<String>) {
    (
        words("Lincoln Jefferson Washington Roosevelt Kennedy Central Eastern Western Northern Southern Riverside Lakeview Hillcrest Oakwood Pinecrest Westfield Fairview Heritage Liberty Union Franklin Madison Monroe Jackson Grant Lee Carver Douglass Dunbar Truman Eisenhower St._Mary's St._Joseph's Holy_Cross Our_Lady Christ_the_King Sacred_Heart Mount_Carmel Bishop_McNamara"),
        words("High Prep Academy Collegiate Catholic_High Tech_High"),
    )
}

pub fn college_names() -> Vec<(String, String)> {
    // (school name, nickname)
    let v: &[(&str, &str)] = &[
        ("Northern Plains", "Stampede"), ("Eastern Lakes", "Voyageurs"), ("Bayou State", "Gators"), ("Capitol Tech", "Senators"), ("Redwood State", "Lumberjacks"), ("Lone Star Christian", "Horned Toads"),
        ("Appalachian Polytechnic", "Miners"), ("Great River", "Riverboats"), ("Blue Ridge", "Mountaineers"), ("Gulf Coast", "Pelicans"), ("Pacific Heights", "Sea Lions"), ("Highland", "Thistles"),
        ("Midland", "Meadowlarks"), ("Crescent City", "Jesters"), ("Heartland State", "Harvesters"), ("Old Dominion Valley", "Cavaliers"), ("Cascade", "Evergreens"), ("Desert Southwest", "Roadrunners"),
        ("Keystone", "Ironmen"), ("Chesapeake", "Watermen"), ("Palmetto", "Herons"), ("Bluegrass", "Colonels"), ("Rust Belt", "Foundry"), ("Great Plains A&M", "Bison"),
        ("Tidewater", "Admirals"), ("Magnolia", "Belles"), ("Granite State", "Quarrymen"), ("Prairie", "Coyotes"), ("Sunbelt", "Scorpions"), ("Olympic", "Torchbearers"),
        ("Sierra", "Grizzlies"), ("Ozark", "Razorbacks"), ("Piedmont", "Textilers"), ("Delta State", "Bluesmen"), ("Rocky Mountain", "Elk"), ("Inland Empire", "Orchardists"),
        ("Harbor City", "Dockers"), ("Monument", "Founders"), ("Cardinal", "Redbirds"), ("Gateway", "Arches"), ("Hoosier Central", "Cornhuskers"), ("Lakeshore", "Lighthouse"),
        ("Bay Area Tech", "Engineers"), ("Space Coast", "Rockets"), ("Rio Grande", "Vaqueros"), ("Cumberland", "Pioneers"), ("Mohawk", "Wolverines"), ("Shenandoah", "Valleymen"),
        ("Yankee", "Minutemen"), ("Alamo", "Defenders"), ("Silver State", "Prospectors"), ("Emerald City", "Rainmakers"), ("Peach State", "Gamecocks"), ("Smoky Mountain", "Bears"),
        ("Colonial", "Patriots"), ("Frontier", "Rangers"), ("Capital Hill", "Lobbyists"), ("Bayside", "Tritons"), ("Mesa", "Sunhawks"), ("Twin Rivers", "Otters"),
        ("Atlantic", "Sailors"), ("Metro State", "Commuters"), ("Hudson", "Highlanders"), ("Ohio Valley", "Buckeyes"), ("Blue Water", "Mariners"), ("Sooner", "Wranglers"),
        ("North Star", "Voyagers"), ("Crossroads", "Railsplitters"), ("Summit", "Climbers"), ("Heritage Christian", "Crusaders"), ("St. Anselm's", "Monks"), ("Our Lady of the Lake", "Lakers"),
        ("Saint Bartholomew", "Friars"), ("Bishop's", "Mitres"), ("Trinity", "Triads"), ("Assumption", "Angels"), ("Xavier-Loyola", "Jesuits"), ("Gonzaga Hills", "Zags"),
        ("Southern Methodist Poly", "Mustangers"), ("Texas Tech Valley", "Raiders"), ("Mountain West", "Rams"), ("Valley Forge", "Continentals"), ("Everglades", "Gators"), ("Dixie", "Rebels"),
        ("Hawkeye", "Hawks"), ("Badger State", "Badgers"), ("Wildcat", "Wildcats"), ("Orange Grove", "Citrus"), ("Cotton Belt", "Boll Weevils"), ("Marshland", "Muskrats"),
        ("Iron Range", "Taconites"), ("Copper Basin", "Smelters"), ("Salt Flats", "Brine Shrimp"), ("Tundra", "Huskies"), ("Glacier", "Icemen"), ("Canyon", "Condors"),
        ("Plateau", "Mesas"), ("Meridian", "Navigators"), ("Zenith", "Aviators"), ("Beacon", "Lamplighters"), ("Cornerstone", "Masons"), ("Liberty Bell", "Ringers"),
    ];
    v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

pub fn conference_names() -> Vec<String> {
    words("Atlantic_Coast_Alliance Big_River Mountain_States Pacific_Conference Lakes_Conference Southern_Athletic Heartland_Conference Eastern_Seaboard Great_Plains Gulf_States Metro_Athletic Valley_Conference Frontier_Conference Coastal_Conference Midwest_Union Keystone_League")
}

pub fn club_suffixes() -> Vec<String> {
    words("BC Basket Hoops Stars Kings Eagles Lions Wolves Giants Titans Spartans Olympians Sharks Falcons Flames")
}

/// Pick a random name from a pool.
pub fn random_name(rng: &mut Rng, pool: &NamePool) -> (String, String) {
    (rng.pick(&pool.first).clone(), rng.pick(&pool.last).clone())
}

/// Weight of a country in a year (linear interpolation of its curve).
pub fn country_weight(c: &Country, year: i32) -> f64 {
    crate::era::interp(&c.weight, year as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countries_reference_real_pools() {
        let pools = builtin_pools();
        for c in builtin_countries() {
            assert!(pools.iter().any(|p| p.id == c.pool), "{} uses unknown pool {}", c.code, c.pool);
            assert!(!c.towns.is_empty());
        }
        for p in &pools {
            assert!(p.first.len() >= 10 && p.last.len() >= 10, "pool {} too small", p.id);
        }
    }

    #[test]
    fn enough_colleges() {
        assert!(college_names().len() >= 90);
    }
}
