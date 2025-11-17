use parabellum_core::GameError;
use parabellum_types::{
    army::UnitName,
    common::{ResearchCost, ResourceGroup},
};

pub type SmithyUpgrades = [u8; 8];

/// Represents the 20 levels of upgrades for each unit.
#[derive(Debug, Clone)]
pub struct SmithyUnitUpgrades {
    pub unit: UnitName,
    pub costs_per_level: [ResearchCost; 20],
}

/// Returns unit smithy upgrade cost for a given level.
pub fn smithy_upgrade_cost_for_unit(
    unit_name: &UnitName,
    level: u8,
) -> Result<ResearchCost, GameError> {
    if level > 20 {
        return Err(GameError::InvalidSmithyLevel(level));
    }

    match smithy_upgrades_for_unit(unit_name) {
        Some(upgrades) => Ok(upgrades.costs_per_level[level as usize].clone()),
        None => Err(GameError::UnitNotFound(unit_name.clone())),
    }
}

/// Returns smithy unit upgrades for a given unit.
fn smithy_upgrades_for_unit<'a>(unit_name: &UnitName) -> Option<&'a SmithyUnitUpgrades> {
    SMITHY_UPGRADES.iter().find(|su| su.unit == *unit_name)
}

static SMITHY_UPGRADES: [SmithyUnitUpgrades; 24] = [
    // 1. Legionnaire ($ab1)
    SmithyUnitUpgrades {
        unit: UnitName::Legionnaire,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(940, 800, 1250, 370),
                time: 6600,
            },
            ResearchCost {
                resources: ResourceGroup(1635, 1395, 2175, 645),
                time: 11491,
            },
            ResearchCost {
                resources: ResourceGroup(2265, 1925, 3010, 890),
                time: 15894,
            },
            ResearchCost {
                resources: ResourceGroup(2850, 2425, 3790, 1120),
                time: 20007,
            },
            ResearchCost {
                resources: ResourceGroup(3405, 2900, 4530, 1340),
                time: 23918,
            },
            ResearchCost {
                resources: ResourceGroup(3940, 3355, 5240, 1550),
                time: 27674,
            },
            ResearchCost {
                resources: ResourceGroup(4460, 3795, 5930, 1755),
                time: 31306,
            },
            ResearchCost {
                resources: ResourceGroup(4960, 4220, 6600, 1955),
                time: 34835,
            },
            ResearchCost {
                resources: ResourceGroup(5450, 4640, 7250, 2145),
                time: 38277,
            },
            ResearchCost {
                resources: ResourceGroup(5930, 5050, 7885, 2335),
                time: 41643,
            },
            ResearchCost {
                resources: ResourceGroup(6400, 5450, 8510, 2520),
                time: 44943,
            },
            ResearchCost {
                resources: ResourceGroup(6860, 5840, 9125, 2700),
                time: 48182,
            },
            ResearchCost {
                resources: ResourceGroup(7315, 6225, 9730, 2880),
                time: 51369,
            },
            ResearchCost {
                resources: ResourceGroup(7765, 6605, 10325, 3055),
                time: 54506,
            },
            ResearchCost {
                resources: ResourceGroup(8205, 6980, 10910, 3230),
                time: 57599,
            },
            ResearchCost {
                resources: ResourceGroup(8640, 7350, 11485, 3400),
                time: 60651,
            },
            ResearchCost {
                resources: ResourceGroup(9065, 7715, 12060, 3570),
                time: 63665,
            },
            ResearchCost {
                resources: ResourceGroup(9490, 8080, 12620, 3735),
                time: 66644,
            },
            ResearchCost {
                resources: ResourceGroup(9910, 8435, 13180, 3900),
                time: 69590,
            },
            ResearchCost {
                resources: ResourceGroup(10325, 8790, 13730, 4065),
                time: 72505,
            },
        ],
    },
    // 2. Praetorian ($ab2)
    SmithyUnitUpgrades {
        unit: UnitName::Praetorian,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(800, 1010, 1320, 650),
                time: 7080,
            },
            ResearchCost {
                resources: ResourceGroup(1395, 1760, 2300, 1130),
                time: 12327,
            },
            ResearchCost {
                resources: ResourceGroup(1925, 2430, 3180, 1565),
                time: 17050,
            },
            ResearchCost {
                resources: ResourceGroup(2425, 3060, 4000, 1970),
                time: 21463,
            },
            ResearchCost {
                resources: ResourceGroup(2900, 3660, 4785, 2355),
                time: 25657,
            },
            ResearchCost {
                resources: ResourceGroup(3355, 4235, 5535, 2725),
                time: 29686,
            },
            ResearchCost {
                resources: ResourceGroup(3795, 4790, 6260, 3085),
                time: 33582,
            },
            ResearchCost {
                resources: ResourceGroup(4220, 5330, 6965, 3430),
                time: 37368,
            },
            ResearchCost {
                resources: ResourceGroup(4640, 5860, 7655, 3770),
                time: 41061,
            },
            ResearchCost {
                resources: ResourceGroup(5050, 6375, 8330, 4100),
                time: 44672,
            },
            ResearchCost {
                resources: ResourceGroup(5450, 6880, 8990, 4425),
                time: 48211,
            },
            ResearchCost {
                resources: ResourceGroup(5840, 7375, 9635, 4745),
                time: 51687,
            },
            ResearchCost {
                resources: ResourceGroup(6225, 7860, 10275, 5060),
                time: 55105,
            },
            ResearchCost {
                resources: ResourceGroup(6605, 8340, 10900, 5370),
                time: 58470,
            },
            ResearchCost {
                resources: ResourceGroup(6980, 8815, 11520, 5675),
                time: 61788,
            },
            ResearchCost {
                resources: ResourceGroup(7350, 9280, 12130, 5975),
                time: 65062,
            },
            ResearchCost {
                resources: ResourceGroup(7715, 9745, 12735, 6270),
                time: 68296,
            },
            ResearchCost {
                resources: ResourceGroup(8080, 10200, 13330, 6565),
                time: 71491,
            },
            ResearchCost {
                resources: ResourceGroup(8435, 10650, 13920, 6855),
                time: 74651,
            },
            ResearchCost {
                resources: ResourceGroup(8790, 11095, 14500, 7140),
                time: 77778,
            },
        ],
    },
    // 3. Imperian ($ab3)
    SmithyUnitUpgrades {
        unit: UnitName::Imperian,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1150, 1220, 1670, 720),
                time: 7560,
            },
            ResearchCost {
                resources: ResourceGroup(2000, 2125, 2910, 1255),
                time: 13163,
            },
            ResearchCost {
                resources: ResourceGroup(2770, 2940, 4020, 1735),
                time: 18206,
            },
            ResearchCost {
                resources: ResourceGroup(3485, 3700, 5060, 2185),
                time: 22918,
            },
            ResearchCost {
                resources: ResourceGroup(4165, 4420, 6050, 2610),
                time: 27397,
            },
            ResearchCost {
                resources: ResourceGroup(4820, 5115, 7000, 3020),
                time: 31699,
            },
            ResearchCost {
                resources: ResourceGroup(5455, 5785, 7920, 3415),
                time: 35859,
            },
            ResearchCost {
                resources: ResourceGroup(6070, 6440, 8815, 3800),
                time: 39902,
            },
            ResearchCost {
                resources: ResourceGroup(6670, 7075, 9685, 4175),
                time: 43845,
            },
            ResearchCost {
                resources: ResourceGroup(7255, 7700, 10535, 4545),
                time: 47700,
            },
            ResearchCost {
                resources: ResourceGroup(7830, 8310, 11370, 4905),
                time: 51480,
            },
            ResearchCost {
                resources: ResourceGroup(8395, 8905, 12190, 5255),
                time: 55191,
            },
            ResearchCost {
                resources: ResourceGroup(8950, 9495, 13000, 5605),
                time: 58841,
            },
            ResearchCost {
                resources: ResourceGroup(9495, 10075, 13790, 5945),
                time: 62434,
            },
            ResearchCost {
                resources: ResourceGroup(10035, 10645, 14575, 6285),
                time: 65977,
            },
            ResearchCost {
                resources: ResourceGroup(10570, 11210, 15345, 6615),
                time: 69473,
            },
            ResearchCost {
                resources: ResourceGroup(11095, 11770, 16110, 6945),
                time: 72926,
            },
            ResearchCost {
                resources: ResourceGroup(11610, 12320, 16865, 7270),
                time: 76338,
            },
            ResearchCost {
                resources: ResourceGroup(12125, 12865, 17610, 7590),
                time: 79712,
            },
            ResearchCost {
                resources: ResourceGroup(12635, 13400, 18345, 7910),
                time: 83051,
            },
        ],
    },
    // 4. Equites Legati ($ab4)
    SmithyUnitUpgrades {
        unit: UnitName::EquitesLegati,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(540, 610, 170, 220),
                time: 5880,
            },
            ResearchCost {
                resources: ResourceGroup(940, 1060, 295, 385),
                time: 10238,
            },
            ResearchCost {
                resources: ResourceGroup(1300, 1470, 410, 530),
                time: 14160,
            },
            ResearchCost {
                resources: ResourceGroup(1635, 1850, 515, 665),
                time: 17825,
            },
            ResearchCost {
                resources: ResourceGroup(1955, 2210, 615, 795),
                time: 21309,
            },
            ResearchCost {
                resources: ResourceGroup(2265, 2560, 715, 920),
                time: 24655,
            },
            ResearchCost {
                resources: ResourceGroup(2560, 2895, 805, 1045),
                time: 27890,
            },
            ResearchCost {
                resources: ResourceGroup(2850, 3220, 895, 1160),
                time: 31035,
            },
            ResearchCost {
                resources: ResourceGroup(3130, 3540, 985, 1275),
                time: 34101,
            },
            ResearchCost {
                resources: ResourceGroup(3405, 3850, 1075, 1390),
                time: 37100,
            },
            ResearchCost {
                resources: ResourceGroup(3675, 4155, 1160, 1500),
                time: 40040,
            },
            ResearchCost {
                resources: ResourceGroup(3940, 4455, 1240, 1605),
                time: 42926,
            },
            ResearchCost {
                resources: ResourceGroup(4205, 4750, 1325, 1710),
                time: 45765,
            },
            ResearchCost {
                resources: ResourceGroup(4460, 5040, 1405, 1815),
                time: 48560,
            },
            ResearchCost {
                resources: ResourceGroup(4715, 5325, 1485, 1920),
                time: 51316,
            },
            ResearchCost {
                resources: ResourceGroup(4960, 5605, 1560, 2020),
                time: 54035,
            },
            ResearchCost {
                resources: ResourceGroup(5210, 5885, 1640, 2120),
                time: 56720,
            },
            ResearchCost {
                resources: ResourceGroup(5455, 6160, 1715, 2220),
                time: 59374,
            },
            ResearchCost {
                resources: ResourceGroup(5695, 6430, 1790, 2320),
                time: 61998,
            },
            ResearchCost {
                resources: ResourceGroup(5930, 6700, 1870, 2415),
                time: 64595,
            },
        ],
    },
    // 5. Equites Imperatoris ($ab5)
    SmithyUnitUpgrades {
        unit: UnitName::EquitesImperatoris,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1315, 1060, 815, 285),
                time: 9720,
            },
            ResearchCost {
                resources: ResourceGroup(2290, 1845, 1415, 500),
                time: 16924,
            },
            ResearchCost {
                resources: ResourceGroup(3170, 2555, 1960, 690),
                time: 23408,
            },
            ResearchCost {
                resources: ResourceGroup(3990, 3215, 2465, 870),
                time: 29466,
            },
            ResearchCost {
                resources: ResourceGroup(4770, 3840, 2945, 1040),
                time: 35224,
            },
            ResearchCost {
                resources: ResourceGroup(5520, 4445, 3410, 1200),
                time: 40756,
            },
            ResearchCost {
                resources: ResourceGroup(6245, 5030, 3860, 1360),
                time: 46105,
            },
            ResearchCost {
                resources: ResourceGroup(6950, 5595, 4295, 1515),
                time: 51302,
            },
            ResearchCost {
                resources: ResourceGroup(7635, 6150, 4715, 1665),
                time: 56372,
            },
            ResearchCost {
                resources: ResourceGroup(8310, 6690, 5130, 1810),
                time: 61329,
            },
            ResearchCost {
                resources: ResourceGroup(8965, 7220, 5540, 1950),
                time: 66188,
            },
            ResearchCost {
                resources: ResourceGroup(9610, 7740, 5940, 2095),
                time: 70960,
            },
            ResearchCost {
                resources: ResourceGroup(10250, 8250, 6330, 2230),
                time: 75652,
            },
            ResearchCost {
                resources: ResourceGroup(10875, 8755, 6715, 2365),
                time: 80273,
            },
            ResearchCost {
                resources: ResourceGroup(11490, 9250, 7100, 2500),
                time: 84828,
            },
            ResearchCost {
                resources: ResourceGroup(12100, 9740, 7475, 2635),
                time: 89323,
            },
            ResearchCost {
                resources: ResourceGroup(12700, 10225, 7845, 2765),
                time: 93762,
            },
            ResearchCost {
                resources: ResourceGroup(13295, 10705, 8215, 2895),
                time: 98149,
            },
            ResearchCost {
                resources: ResourceGroup(13885, 11175, 8575, 3025),
                time: 102487,
            },
            ResearchCost {
                resources: ResourceGroup(14465, 11645, 8935, 3150),
                time: 106780,
            },
        ],
    },
    // 6. Equites Caesaris ($ab6)
    SmithyUnitUpgrades {
        unit: UnitName::EquitesCaesaris,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(990, 1145, 1450, 355),
                time: 12360,
            },
            ResearchCost {
                resources: ResourceGroup(1720, 1995, 2525, 620),
                time: 21520,
            },
            ResearchCost {
                resources: ResourceGroup(2380, 2755, 3490, 855),
                time: 29766,
            },
            ResearchCost {
                resources: ResourceGroup(2995, 3470, 4395, 1075),
                time: 37469,
            },
            ResearchCost {
                resources: ResourceGroup(3580, 4150, 5255, 1285),
                time: 44791,
            },
            ResearchCost {
                resources: ResourceGroup(4140, 4800, 6080, 1490),
                time: 51825,
            },
            ResearchCost {
                resources: ResourceGroup(4685, 5430, 6880, 1685),
                time: 58627,
            },
            ResearchCost {
                resources: ResourceGroup(5210, 6045, 7655, 1875),
                time: 65236,
            },
            ResearchCost {
                resources: ResourceGroup(5725, 6640, 8410, 2060),
                time: 71682,
            },
            ResearchCost {
                resources: ResourceGroup(6230, 7225, 9150, 2240),
                time: 77986,
            },
            ResearchCost {
                resources: ResourceGroup(6725, 7795, 9875, 2415),
                time: 84165,
            },
            ResearchCost {
                resources: ResourceGroup(7210, 8360, 10585, 2590),
                time: 90233,
            },
            ResearchCost {
                resources: ResourceGroup(7685, 8910, 11285, 2765),
                time: 96200,
            },
            ResearchCost {
                resources: ResourceGroup(8155, 9455, 11975, 2930),
                time: 102075,
            },
            ResearchCost {
                resources: ResourceGroup(8620, 9995, 12655, 3100),
                time: 107868,
            },
            ResearchCost {
                resources: ResourceGroup(9075, 10520, 13325, 3260),
                time: 113583,
            },
            ResearchCost {
                resources: ResourceGroup(9525, 11045, 13985, 3425),
                time: 119228,
            },
            ResearchCost {
                resources: ResourceGroup(9970, 11560, 14640, 3585),
                time: 124806,
            },
            ResearchCost {
                resources: ResourceGroup(10410, 12075, 15290, 3745),
                time: 130323,
            },
            ResearchCost {
                resources: ResourceGroup(10850, 12580, 15930, 3900),
                time: 135782,
            },
        ],
    },
    // 7. Battering Ram (Roman) ($ab7)
    SmithyUnitUpgrades {
        unit: UnitName::BatteringRam,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(2135, 875, 1235, 215),
                time: 15600,
            },
            ResearchCost {
                resources: ResourceGroup(3715, 1520, 2145, 375),
                time: 27161,
            },
            ResearchCost {
                resources: ResourceGroup(5140, 2105, 2970, 520),
                time: 37568,
            },
            ResearchCost {
                resources: ResourceGroup(6465, 2645, 3740, 655),
                time: 47290,
            },
            ResearchCost {
                resources: ResourceGroup(7730, 3165, 4470, 785),
                time: 56533,
            },
            ResearchCost {
                resources: ResourceGroup(8945, 3660, 5170, 910),
                time: 65410,
            },
            ResearchCost {
                resources: ResourceGroup(10120, 4140, 5850, 1030),
                time: 73995,
            },
            ResearchCost {
                resources: ResourceGroup(11260, 4610, 6510, 1145),
                time: 82337,
            },
            ResearchCost {
                resources: ResourceGroup(12370, 5065, 7155, 1255),
                time: 90473,
            },
            ResearchCost {
                resources: ResourceGroup(13460, 5510, 7780, 1365),
                time: 98429,
            },
            ResearchCost {
                resources: ResourceGroup(14525, 5945, 8400, 1475),
                time: 106228,
            },
            ResearchCost {
                resources: ResourceGroup(15575, 6375, 9005, 1580),
                time: 113886,
            },
            ResearchCost {
                resources: ResourceGroup(16605, 6795, 9600, 1685),
                time: 121417,
            },
            ResearchCost {
                resources: ResourceGroup(17620, 7210, 10185, 1790),
                time: 128833,
            },
            ResearchCost {
                resources: ResourceGroup(18620, 7620, 10765, 1890),
                time: 136144,
            },
            ResearchCost {
                resources: ResourceGroup(19605, 8025, 11335, 1990),
                time: 143358,
            },
            ResearchCost {
                resources: ResourceGroup(20580, 8425, 11895, 2090),
                time: 150482,
            },
            ResearchCost {
                resources: ResourceGroup(21540, 8820, 12455, 2190),
                time: 157523,
            },
            ResearchCost {
                resources: ResourceGroup(22495, 9210, 13005, 2285),
                time: 164485,
            },
            ResearchCost {
                resources: ResourceGroup(23435, 9595, 13550, 2380),
                time: 171375,
            },
        ],
    },
    // 8. Fire Catapult ($ab8)
    SmithyUnitUpgrades {
        unit: UnitName::FireCatapult,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1125, 1590, 735, 130),
                time: 28800,
            },
            ResearchCost {
                resources: ResourceGroup(1960, 2770, 1275, 230),
                time: 50144,
            },
            ResearchCost {
                resources: ResourceGroup(2710, 3835, 1765, 315),
                time: 69357,
            },
            ResearchCost {
                resources: ResourceGroup(3410, 4825, 2225, 400),
                time: 87305,
            },
            ResearchCost {
                resources: ResourceGroup(4075, 5770, 2660, 475),
                time: 104368,
            },
            ResearchCost {
                resources: ResourceGroup(4715, 6675, 3075, 550),
                time: 120757,
            },
            ResearchCost {
                resources: ResourceGroup(5335, 7550, 3480, 625),
                time: 136606,
            },
            ResearchCost {
                resources: ResourceGroup(5940, 8400, 3870, 695),
                time: 152007,
            },
            ResearchCost {
                resources: ResourceGroup(6525, 9230, 4255, 765),
                time: 167027,
            },
            ResearchCost {
                resources: ResourceGroup(7100, 10045, 4625, 830),
                time: 181716,
            },
            ResearchCost {
                resources: ResourceGroup(7660, 10840, 4995, 895),
                time: 196113,
            },
            ResearchCost {
                resources: ResourceGroup(8215, 11620, 5355, 960),
                time: 210251,
            },
            ResearchCost {
                resources: ResourceGroup(8755, 12390, 5710, 1025),
                time: 224154,
            },
            ResearchCost {
                resources: ResourceGroup(9290, 13145, 6055, 1085),
                time: 237845,
            },
            ResearchCost {
                resources: ResourceGroup(9820, 13890, 6400, 1150),
                time: 251342,
            },
            ResearchCost {
                resources: ResourceGroup(10340, 14625, 6740, 1210),
                time: 264660,
            },
            ResearchCost {
                resources: ResourceGroup(10850, 15355, 7075, 1270),
                time: 277812,
            },
            ResearchCost {
                resources: ResourceGroup(11360, 16070, 7405, 1330),
                time: 290811,
            },
            ResearchCost {
                resources: ResourceGroup(11860, 16780, 7730, 1390),
                time: 303665,
            },
            ResearchCost {
                resources: ResourceGroup(12360, 17485, 8055, 1445),
                time: 316385,
            },
        ],
    },
    // 1. Clubswinger ($ab11)
    SmithyUnitUpgrades {
        unit: UnitName::Maceman,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(765, 625, 480, 440),
                time: 3960,
            },
            ResearchCost {
                resources: ResourceGroup(1330, 1090, 835, 765),
                time: 6895,
            },
            ResearchCost {
                resources: ResourceGroup(1840, 1505, 1155, 1060),
                time: 9537,
            },
            ResearchCost {
                resources: ResourceGroup(2320, 1895, 1455, 1335),
                time: 12004,
            },
            ResearchCost {
                resources: ResourceGroup(2770, 2265, 1740, 1595),
                time: 14351,
            },
            ResearchCost {
                resources: ResourceGroup(3210, 2620, 2015, 1845),
                time: 16604,
            },
            ResearchCost {
                resources: ResourceGroup(3630, 2965, 2275, 2085),
                time: 18783,
            },
            ResearchCost {
                resources: ResourceGroup(4040, 3300, 2535, 2320),
                time: 20901,
            },
            ResearchCost {
                resources: ResourceGroup(4435, 3625, 2785, 2550),
                time: 22966,
            },
            ResearchCost {
                resources: ResourceGroup(4825, 3945, 3030, 2775),
                time: 24986,
            },
            ResearchCost {
                resources: ResourceGroup(5210, 4255, 3270, 2995),
                time: 26966,
            },
            ResearchCost {
                resources: ResourceGroup(5585, 4565, 3505, 3210),
                time: 28909,
            },
            ResearchCost {
                resources: ResourceGroup(5955, 4865, 3735, 3425),
                time: 30821,
            },
            ResearchCost {
                resources: ResourceGroup(6320, 5160, 3965, 3635),
                time: 32704,
            },
            ResearchCost {
                resources: ResourceGroup(6675, 5455, 4190, 3840),
                time: 34560,
            },
            ResearchCost {
                resources: ResourceGroup(7030, 5745, 4410, 4045),
                time: 36391,
            },
            ResearchCost {
                resources: ResourceGroup(7380, 6030, 4630, 4245),
                time: 38199,
            },
            ResearchCost {
                resources: ResourceGroup(7725, 6310, 4845, 4445),
                time: 39986,
            },
            ResearchCost {
                resources: ResourceGroup(8065, 6590, 5060, 4640),
                time: 41754,
            },
            ResearchCost {
                resources: ResourceGroup(8405, 6865, 5275, 4835),
                time: 43503,
            },
        ],
    },
    // 2. Spearman ($ab12)
    SmithyUnitUpgrades {
        unit: UnitName::Spearman,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1115, 590, 795, 440),
                time: 5160,
            },
            ResearchCost {
                resources: ResourceGroup(1940, 1025, 1385, 765),
                time: 8984,
            },
            ResearchCost {
                resources: ResourceGroup(2685, 1420, 1915, 1060),
                time: 12426,
            },
            ResearchCost {
                resources: ResourceGroup(3380, 1790, 2410, 1335),
                time: 15642,
            },
            ResearchCost {
                resources: ResourceGroup(4040, 2140, 2880, 1595),
                time: 18699,
            },
            ResearchCost {
                resources: ResourceGroup(4675, 2475, 3335, 1845),
                time: 21636,
            },
            ResearchCost {
                resources: ResourceGroup(5290, 2800, 3770, 2085),
                time: 24475,
            },
            ResearchCost {
                resources: ResourceGroup(5885, 3115, 4195, 2320),
                time: 27235,
            },
            ResearchCost {
                resources: ResourceGroup(6465, 3420, 4610, 2550),
                time: 29926,
            },
            ResearchCost {
                resources: ResourceGroup(7035, 3725, 5015, 2775),
                time: 32557,
            },
            ResearchCost {
                resources: ResourceGroup(7595, 4020, 5415, 2995),
                time: 35137,
            },
            ResearchCost {
                resources: ResourceGroup(8140, 4305, 5805, 3210),
                time: 37670,
            },
            ResearchCost {
                resources: ResourceGroup(8680, 4590, 6190, 3425),
                time: 40161,
            },
            ResearchCost {
                resources: ResourceGroup(9210, 4875, 6565, 3635),
                time: 42614,
            },
            ResearchCost {
                resources: ResourceGroup(9730, 5150, 6940, 3840),
                time: 45032,
            },
            ResearchCost {
                resources: ResourceGroup(10245, 5420, 7305, 4045),
                time: 47418,
            },
            ResearchCost {
                resources: ResourceGroup(10755, 5690, 7670, 4245),
                time: 49775,
            },
            ResearchCost {
                resources: ResourceGroup(11260, 5960, 8030, 4445),
                time: 52104,
            },
            ResearchCost {
                resources: ResourceGroup(11755, 6220, 8380, 4640),
                time: 54407,
            },
            ResearchCost {
                resources: ResourceGroup(12250, 6480, 8735, 4835),
                time: 56686,
            },
        ],
    },
    // 3. Axeman ($ab13)
    SmithyUnitUpgrades {
        unit: UnitName::Axeman,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1010, 940, 1390, 650),
                time: 5400,
            },
            ResearchCost {
                resources: ResourceGroup(1760, 1635, 2420, 1130),
                time: 9402,
            },
            ResearchCost {
                resources: ResourceGroup(2430, 2265, 3345, 1565),
                time: 13004,
            },
            ResearchCost {
                resources: ResourceGroup(3060, 2850, 4215, 1970),
                time: 16370,
            },
            ResearchCost {
                resources: ResourceGroup(3660, 3405, 5035, 2355),
                time: 19569,
            },
            ResearchCost {
                resources: ResourceGroup(4235, 3940, 5830, 2725),
                time: 22642,
            },
            ResearchCost {
                resources: ResourceGroup(4790, 4460, 6595, 3085),
                time: 25614,
            },
            ResearchCost {
                resources: ResourceGroup(5330, 4960, 7335, 3430),
                time: 28501,
            },
            ResearchCost {
                resources: ResourceGroup(5860, 5450, 8060, 3770),
                time: 31318,
            },
            ResearchCost {
                resources: ResourceGroup(6375, 5930, 8770, 4100),
                time: 34072,
            },
            ResearchCost {
                resources: ResourceGroup(6880, 6400, 9465, 4425),
                time: 36771,
            },
            ResearchCost {
                resources: ResourceGroup(7375, 6860, 10150, 4745),
                time: 39422,
            },
            ResearchCost {
                resources: ResourceGroup(7860, 7315, 10820, 5060),
                time: 42029,
            },
            ResearchCost {
                resources: ResourceGroup(8340, 7765, 11480, 5370),
                time: 44596,
            },
            ResearchCost {
                resources: ResourceGroup(8815, 8205, 12130, 5675),
                time: 47127,
            },
            ResearchCost {
                resources: ResourceGroup(9280, 8640, 12775, 5975),
                time: 49624,
            },
            ResearchCost {
                resources: ResourceGroup(9745, 9065, 13410, 6270),
                time: 52090,
            },
            ResearchCost {
                resources: ResourceGroup(10200, 9490, 14035, 6565),
                time: 54527,
            },
            ResearchCost {
                resources: ResourceGroup(10650, 9910, 14655, 6855),
                time: 56937,
            },
            ResearchCost {
                resources: ResourceGroup(11095, 10325, 15270, 7140),
                time: 59322,
            },
        ],
    },
    // 4. Scout ($ab14)
    SmithyUnitUpgrades {
        unit: UnitName::Scout,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1220, 800, 550, 510),
                time: 5160,
            },
            ResearchCost {
                resources: ResourceGroup(2125, 1395, 960, 890),
                time: 8984,
            },
            ResearchCost {
                resources: ResourceGroup(2940, 1925, 1325, 1230),
                time: 12426,
            },
            ResearchCost {
                resources: ResourceGroup(3700, 2425, 1665, 1545),
                time: 15642,
            },
            ResearchCost {
                resources: ResourceGroup(4420, 2900, 1995, 1850),
                time: 18699,
            },
            ResearchCost {
                resources: ResourceGroup(5115, 3355, 2305, 2140),
                time: 21636,
            },
            ResearchCost {
                resources: ResourceGroup(5785, 3795, 2610, 2420),
                time: 24475,
            },
            ResearchCost {
                resources: ResourceGroup(6440, 4220, 2905, 2690),
                time: 27235,
            },
            ResearchCost {
                resources: ResourceGroup(7075, 4640, 3190, 2960),
                time: 29926,
            },
            ResearchCost {
                resources: ResourceGroup(7700, 5050, 3470, 3220),
                time: 32557,
            },
            ResearchCost {
                resources: ResourceGroup(8310, 5450, 3745, 3475),
                time: 35137,
            },
            ResearchCost {
                resources: ResourceGroup(8905, 5840, 4015, 3725),
                time: 37670,
            },
            ResearchCost {
                resources: ResourceGroup(9495, 6225, 4280, 3970),
                time: 40161,
            },
            ResearchCost {
                resources: ResourceGroup(10075, 6605, 4540, 4210),
                time: 42614,
            },
            ResearchCost {
                resources: ResourceGroup(10645, 6980, 4800, 4450),
                time: 45032,
            },
            ResearchCost {
                resources: ResourceGroup(11210, 7350, 5055, 4685),
                time: 47418,
            },
            ResearchCost {
                resources: ResourceGroup(11770, 7715, 5305, 4920),
                time: 49775,
            },
            ResearchCost {
                resources: ResourceGroup(12320, 8080, 5555, 5150),
                time: 52104,
            },
            ResearchCost {
                resources: ResourceGroup(12865, 8435, 5800, 5375),
                time: 54407,
            },
            ResearchCost {
                resources: ResourceGroup(13400, 8790, 6040, 5605),
                time: 56686,
            },
        ],
    },
    // 5. Paladin ($ab15)
    SmithyUnitUpgrades {
        unit: UnitName::Paladin,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1345, 995, 1115, 345),
                time: 9000,
            },
            ResearchCost {
                resources: ResourceGroup(2340, 1730, 1940, 595),
                time: 15670,
            },
            ResearchCost {
                resources: ResourceGroup(3240, 2395, 2685, 825),
                time: 21674,
            },
            ResearchCost {
                resources: ResourceGroup(4075, 3015, 3380, 1040),
                time: 27283,
            },
            ResearchCost {
                resources: ResourceGroup(4875, 3605, 4040, 1240),
                time: 32615,
            },
            ResearchCost {
                resources: ResourceGroup(5640, 4170, 4675, 1435),
                time: 37737,
            },
            ResearchCost {
                resources: ResourceGroup(6380, 4720, 5290, 1625),
                time: 42689,
            },
            ResearchCost {
                resources: ResourceGroup(7100, 5250, 5885, 1810),
                time: 47502,
            },
            ResearchCost {
                resources: ResourceGroup(7800, 5770, 6465, 1985),
                time: 52196,
            },
            ResearchCost {
                resources: ResourceGroup(8485, 6280, 7035, 2160),
                time: 56786,
            },
            ResearchCost {
                resources: ResourceGroup(9160, 6775, 7595, 2330),
                time: 61285,
            },
            ResearchCost {
                resources: ResourceGroup(9820, 7265, 8140, 2500),
                time: 65703,
            },
            ResearchCost {
                resources: ResourceGroup(10470, 7745, 8680, 2665),
                time: 70048,
            },
            ResearchCost {
                resources: ResourceGroup(11110, 8215, 9210, 2830),
                time: 74327,
            },
            ResearchCost {
                resources: ResourceGroup(11740, 8685, 9730, 2990),
                time: 78544,
            },
            ResearchCost {
                resources: ResourceGroup(12360, 9145, 10245, 3145),
                time: 82706,
            },
            ResearchCost {
                resources: ResourceGroup(12975, 9600, 10755, 3305),
                time: 86816,
            },
            ResearchCost {
                resources: ResourceGroup(13580, 10045, 11260, 3460),
                time: 90878,
            },
            ResearchCost {
                resources: ResourceGroup(14180, 10490, 11755, 3610),
                time: 94895,
            },
            ResearchCost {
                resources: ResourceGroup(14775, 10930, 12250, 3765),
                time: 98870,
            },
        ],
    },
    // 6. Teutonic Knight ($ab16)
    SmithyUnitUpgrades {
        unit: UnitName::TeutonicKnight,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1085, 1235, 1185, 240),
                time: 10680,
            },
            ResearchCost {
                resources: ResourceGroup(1885, 2150, 2065, 420),
                time: 18595,
            },
            ResearchCost {
                resources: ResourceGroup(2610, 2975, 2860, 580),
                time: 25720,
            },
            ResearchCost {
                resources: ResourceGroup(3285, 3745, 3595, 730),
                time: 32376,
            },
            ResearchCost {
                resources: ResourceGroup(3925, 4475, 4300, 870),
                time: 38703,
            },
            ResearchCost {
                resources: ResourceGroup(4540, 5180, 4975, 1005),
                time: 44781,
            },
            ResearchCost {
                resources: ResourceGroup(5140, 5860, 5630, 1140),
                time: 50658,
            },
            ResearchCost {
                resources: ResourceGroup(5720, 6520, 6265, 1265),
                time: 56369,
            },
            ResearchCost {
                resources: ResourceGroup(6285, 7160, 6880, 1390),
                time: 61939,
            },
            ResearchCost {
                resources: ResourceGroup(6835, 7790, 7485, 1515),
                time: 67386,
            },
            ResearchCost {
                resources: ResourceGroup(7375, 8410, 8080, 1635),
                time: 72725,
            },
            ResearchCost {
                resources: ResourceGroup(7910, 9015, 8665, 1750),
                time: 77968,
            },
            ResearchCost {
                resources: ResourceGroup(8430, 9610, 9235, 1870),
                time: 83124,
            },
            ResearchCost {
                resources: ResourceGroup(8945, 10200, 9800, 1980),
                time: 88201,
            },
            ResearchCost {
                resources: ResourceGroup(9455, 10780, 10355, 2095),
                time: 93206,
            },
            ResearchCost {
                resources: ResourceGroup(9955, 11350, 10905, 2205),
                time: 98145,
            },
            ResearchCost {
                resources: ResourceGroup(10450, 11915, 11445, 2315),
                time: 103022,
            },
            ResearchCost {
                resources: ResourceGroup(10940, 12470, 11980, 2425),
                time: 107842,
            },
            ResearchCost {
                resources: ResourceGroup(11425, 13020, 12510, 2530),
                time: 112609,
            },
            ResearchCost {
                resources: ResourceGroup(11900, 13565, 13035, 2635),
                time: 117326,
            },
        ],
    },
    // 7. Battering Ram (Teuton) ($ab17)
    SmithyUnitUpgrades {
        unit: UnitName::Ram,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(2365, 735, 885, 215),
                time: 14400,
            },
            ResearchCost {
                resources: ResourceGroup(4120, 1275, 1540, 375),
                time: 25072,
            },
            ResearchCost {
                resources: ResourceGroup(5700, 1765, 2125, 520),
                time: 34678,
            },
            ResearchCost {
                resources: ResourceGroup(7175, 2225, 2680, 655),
                time: 43653,
            },
            ResearchCost {
                resources: ResourceGroup(8575, 2660, 3200, 785),
                time: 52184,
            },
            ResearchCost {
                resources: ResourceGroup(9925, 3075, 3705, 910),
                time: 60379,
            },
            ResearchCost {
                resources: ResourceGroup(11225, 3480, 4190, 1030),
                time: 68303,
            },
            ResearchCost {
                resources: ResourceGroup(12490, 3870, 4660, 1145),
                time: 76004,
            },
            ResearchCost {
                resources: ResourceGroup(13725, 4255, 5125, 1255),
                time: 83513,
            },
            ResearchCost {
                resources: ResourceGroup(14935, 4625, 5575, 1365),
                time: 90858,
            },
            ResearchCost {
                resources: ResourceGroup(16115, 4995, 6015, 1475),
                time: 98057,
            },
            ResearchCost {
                resources: ResourceGroup(17280, 5355, 6450, 1580),
                time: 105125,
            },
            ResearchCost {
                resources: ResourceGroup(18420, 5710, 6875, 1685),
                time: 112077,
            },
            ResearchCost {
                resources: ResourceGroup(19545, 6055, 7295, 1790),
                time: 118923,
            },
            ResearchCost {
                resources: ResourceGroup(20655, 6400, 7710, 1890),
                time: 125671,
            },
            ResearchCost {
                resources: ResourceGroup(21750, 6740, 8115, 1990),
                time: 132330,
            },
            ResearchCost {
                resources: ResourceGroup(22830, 7075, 8520, 2090),
                time: 138906,
            },
            ResearchCost {
                resources: ResourceGroup(23900, 7405, 8920, 2190),
                time: 145405,
            },
            ResearchCost {
                resources: ResourceGroup(24955, 7730, 9315, 2285),
                time: 151833,
            },
            ResearchCost {
                resources: ResourceGroup(26000, 8055, 9705, 2380),
                time: 158193,
            },
        ],
    },
    // 8. Catapult (Teuton) ($ab18)
    SmithyUnitUpgrades {
        unit: UnitName::Catapult,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1065, 1415, 735, 95),
                time: 28800,
            },
            ResearchCost {
                resources: ResourceGroup(1855, 2465, 1275, 170),
                time: 50144,
            },
            ResearchCost {
                resources: ResourceGroup(2570, 3410, 1765, 235),
                time: 69357,
            },
            ResearchCost {
                resources: ResourceGroup(3235, 4295, 2225, 295),
                time: 87305,
            },
            ResearchCost {
                resources: ResourceGroup(3865, 5135, 2660, 350),
                time: 104368,
            },
            ResearchCost {
                resources: ResourceGroup(4470, 5940, 3075, 405),
                time: 120757,
            },
            ResearchCost {
                resources: ResourceGroup(5060, 6720, 3480, 460),
                time: 136606,
            },
            ResearchCost {
                resources: ResourceGroup(5630, 7475, 3870, 510),
                time: 152007,
            },
            ResearchCost {
                resources: ResourceGroup(6185, 8215, 4255, 560),
                time: 167027,
            },
            ResearchCost {
                resources: ResourceGroup(6730, 8940, 4625, 610),
                time: 181716,
            },
            ResearchCost {
                resources: ResourceGroup(7265, 9645, 4995, 660),
                time: 196113,
            },
            ResearchCost {
                resources: ResourceGroup(7785, 10340, 5355, 705),
                time: 210251,
            },
            ResearchCost {
                resources: ResourceGroup(8300, 11025, 5710, 750),
                time: 224154,
            },
            ResearchCost {
                resources: ResourceGroup(8810, 11700, 6055, 800),
                time: 237845,
            },
            ResearchCost {
                resources: ResourceGroup(9310, 12365, 6400, 845),
                time: 251342,
            },
            ResearchCost {
                resources: ResourceGroup(9800, 13020, 6740, 890),
                time: 264660,
            },
            ResearchCost {
                resources: ResourceGroup(10290, 13665, 7075, 930),
                time: 277812,
            },
            ResearchCost {
                resources: ResourceGroup(10770, 14305, 7405, 975),
                time: 290811,
            },
            ResearchCost {
                resources: ResourceGroup(11245, 14935, 7730, 1020),
                time: 303665,
            },
            ResearchCost {
                resources: ResourceGroup(11720, 15565, 8055, 1060),
                time: 316385,
            },
        ],
    },
    // 1. Phalanx ($ab21)
    SmithyUnitUpgrades {
        unit: UnitName::Phalanx,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(800, 1010, 585, 370),
                time: 4920,
            },
            ResearchCost {
                resources: ResourceGroup(1395, 1760, 1020, 645),
                time: 8566,
            },
            ResearchCost {
                resources: ResourceGroup(1925, 2430, 1410, 890),
                time: 11848,
            },
            ResearchCost {
                resources: ResourceGroup(2425, 3060, 1775, 1120),
                time: 14915,
            },
            ResearchCost {
                resources: ResourceGroup(2900, 3660, 2120, 1340),
                time: 17830,
            },
            ResearchCost {
                resources: ResourceGroup(3355, 4235, 2455, 1550),
                time: 20629,
            },
            ResearchCost {
                resources: ResourceGroup(3795, 4790, 2775, 1755),
                time: 23337,
            },
            ResearchCost {
                resources: ResourceGroup(4220, 5330, 3090, 1955),
                time: 25968,
            },
            ResearchCost {
                resources: ResourceGroup(4640, 5860, 3395, 2145),
                time: 28534,
            },
            ResearchCost {
                resources: ResourceGroup(5050, 6375, 3690, 2335),
                time: 31043,
            },
            ResearchCost {
                resources: ResourceGroup(5450, 6880, 3985, 2520),
                time: 33503,
            },
            ResearchCost {
                resources: ResourceGroup(5840, 7375, 4270, 2700),
                time: 35918,
            },
            ResearchCost {
                resources: ResourceGroup(6225, 7860, 4555, 2880),
                time: 38293,
            },
            ResearchCost {
                resources: ResourceGroup(6605, 8340, 4830, 3055),
                time: 40632,
            },
            ResearchCost {
                resources: ResourceGroup(6980, 8815, 5105, 3230),
                time: 42938,
            },
            ResearchCost {
                resources: ResourceGroup(7350, 9280, 5375, 3400),
                time: 45213,
            },
            ResearchCost {
                resources: ResourceGroup(7715, 9745, 5645, 3570),
                time: 47460,
            },
            ResearchCost {
                resources: ResourceGroup(8080, 10200, 5905, 3735),
                time: 49680,
            },
            ResearchCost {
                resources: ResourceGroup(8435, 10650, 6170, 3900),
                time: 51876,
            },
            ResearchCost {
                resources: ResourceGroup(8790, 11095, 6425, 4065),
                time: 54049,
            },
        ],
    },
    // 2. Swordsman ($ab22)
    SmithyUnitUpgrades {
        unit: UnitName::Swordsman,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1080, 1150, 1495, 580),
                time: 6120,
            },
            ResearchCost {
                resources: ResourceGroup(1880, 2000, 2605, 1010),
                time: 10656,
            },
            ResearchCost {
                resources: ResourceGroup(2600, 2770, 3600, 1395),
                time: 14738,
            },
            ResearchCost {
                resources: ResourceGroup(3275, 3485, 4530, 1760),
                time: 18552,
            },
            ResearchCost {
                resources: ResourceGroup(3915, 4165, 5420, 2100),
                time: 22178,
            },
            ResearchCost {
                resources: ResourceGroup(4530, 4820, 6270, 2430),
                time: 25661,
            },
            ResearchCost {
                resources: ResourceGroup(5125, 5455, 7090, 2750),
                time: 29029,
            },
            ResearchCost {
                resources: ResourceGroup(5700, 6070, 7890, 3060),
                time: 32302,
            },
            ResearchCost {
                resources: ResourceGroup(6265, 6670, 8670, 3365),
                time: 35493,
            },
            ResearchCost {
                resources: ResourceGroup(6815, 7255, 9435, 3660),
                time: 38615,
            },
            ResearchCost {
                resources: ResourceGroup(7355, 7830, 10180, 3950),
                time: 41674,
            },
            ResearchCost {
                resources: ResourceGroup(7885, 8395, 10915, 4235),
                time: 44678,
            },
            ResearchCost {
                resources: ResourceGroup(8405, 8950, 11635, 4515),
                time: 47633,
            },
            ResearchCost {
                resources: ResourceGroup(8920, 9495, 12345, 4790),
                time: 50542,
            },
            ResearchCost {
                resources: ResourceGroup(9425, 10035, 13045, 5060),
                time: 53410,
            },
            ResearchCost {
                resources: ResourceGroup(9925, 10570, 13740, 5330),
                time: 56240,
            },
            ResearchCost {
                resources: ResourceGroup(10420, 11095, 14420, 5595),
                time: 59035,
            },
            ResearchCost {
                resources: ResourceGroup(10905, 11610, 15095, 5855),
                time: 61797,
            },
            ResearchCost {
                resources: ResourceGroup(11385, 12125, 15765, 6115),
                time: 64529,
            },
            ResearchCost {
                resources: ResourceGroup(11865, 12635, 16425, 6370),
                time: 67232,
            },
        ],
    },
    // 3. Pathfinder ($ab23)
    SmithyUnitUpgrades {
        unit: UnitName::Pathfinder,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(645, 575, 170, 220),
                time: 5880,
            },
            ResearchCost {
                resources: ResourceGroup(1125, 1000, 295, 385),
                time: 10238,
            },
            ResearchCost {
                resources: ResourceGroup(1555, 1385, 410, 530),
                time: 14160,
            },
            ResearchCost {
                resources: ResourceGroup(1955, 1745, 515, 665),
                time: 17825,
            },
            ResearchCost {
                resources: ResourceGroup(2335, 2085, 615, 795),
                time: 21309,
            },
            ResearchCost {
                resources: ResourceGroup(2705, 2410, 715, 920),
                time: 24655,
            },
            ResearchCost {
                resources: ResourceGroup(3060, 2725, 805, 1045),
                time: 27890,
            },
            ResearchCost {
                resources: ResourceGroup(3405, 3035, 895, 1160),
                time: 31035,
            },
            ResearchCost {
                resources: ResourceGroup(3740, 3335, 985, 1275),
                time: 34101,
            },
            ResearchCost {
                resources: ResourceGroup(4070, 3630, 1075, 1390),
                time: 37100,
            },
            ResearchCost {
                resources: ResourceGroup(4390, 3915, 1160, 1500),
                time: 40040,
            },
            ResearchCost {
                resources: ResourceGroup(4710, 4200, 1240, 1605),
                time: 42926,
            },
            ResearchCost {
                resources: ResourceGroup(5020, 4475, 1325, 1710),
                time: 45765,
            },
            ResearchCost {
                resources: ResourceGroup(5325, 4750, 1405, 1815),
                time: 48560,
            },
            ResearchCost {
                resources: ResourceGroup(5630, 5020, 1485, 1920),
                time: 51316,
            },
            ResearchCost {
                resources: ResourceGroup(5925, 5285, 1560, 2020),
                time: 54035,
            },
            ResearchCost {
                resources: ResourceGroup(6220, 5545, 1640, 2120),
                time: 56720,
            },
            ResearchCost {
                resources: ResourceGroup(6515, 5805, 1715, 2220),
                time: 59374,
            },
            ResearchCost {
                resources: ResourceGroup(6800, 6065, 1790, 2320),
                time: 61998,
            },
            ResearchCost {
                resources: ResourceGroup(7085, 6315, 1870, 2415),
                time: 64595,
            },
        ],
    },
    // 4. Theutates Thunder ($ab24)
    SmithyUnitUpgrades {
        unit: UnitName::TheutatesThunder,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1275, 1625, 905, 290),
                time: 9240,
            },
            ResearchCost {
                resources: ResourceGroup(2220, 2830, 1575, 505),
                time: 16088,
            },
            ResearchCost {
                resources: ResourceGroup(3070, 3915, 2180, 700),
                time: 22252,
            },
            ResearchCost {
                resources: ResourceGroup(3865, 4925, 2745, 880),
                time: 28010,
            },
            ResearchCost {
                resources: ResourceGroup(4620, 5890, 3280, 1050),
                time: 33485,
            },
            ResearchCost {
                resources: ResourceGroup(5345, 6815, 3795, 1215),
                time: 38743,
            },
            ResearchCost {
                resources: ResourceGroup(6050, 7710, 4295, 1375),
                time: 43828,
            },
            ResearchCost {
                resources: ResourceGroup(6730, 8575, 4775, 1530),
                time: 48769,
            },
            ResearchCost {
                resources: ResourceGroup(7395, 9425, 5250, 1680),
                time: 53588,
            },
            ResearchCost {
                resources: ResourceGroup(8045, 10255, 5710, 1830),
                time: 58300,
            },
            ResearchCost {
                resources: ResourceGroup(8680, 11065, 6165, 1975),
                time: 62920,
            },
            ResearchCost {
                resources: ResourceGroup(9310, 11865, 6605, 2115),
                time: 67455,
            },
            ResearchCost {
                resources: ResourceGroup(9925, 12650, 7045, 2255),
                time: 71916,
            },
            ResearchCost {
                resources: ResourceGroup(10530, 13420, 7475, 2395),
                time: 76309,
            },
            ResearchCost {
                resources: ResourceGroup(11125, 14180, 7900, 2530),
                time: 80639,
            },
            ResearchCost {
                resources: ResourceGroup(11715, 14935, 8315, 2665),
                time: 84912,
            },
            ResearchCost {
                resources: ResourceGroup(12300, 15675, 8730, 2795),
                time: 89131,
            },
            ResearchCost {
                resources: ResourceGroup(12875, 16410, 9140, 2930),
                time: 93302,
            },
            ResearchCost {
                resources: ResourceGroup(13445, 17135, 9540, 3060),
                time: 97426,
            },
            ResearchCost {
                resources: ResourceGroup(14005, 17850, 9940, 3185),
                time: 101507,
            },
        ],
    },
    // 5. Druidrider ($ab25)
    SmithyUnitUpgrades {
        unit: UnitName::Druidrider,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1310, 1205, 1080, 500),
                time: 9480,
            },
            ResearchCost {
                resources: ResourceGroup(2280, 2100, 1880, 870),
                time: 16506,
            },
            ResearchCost {
                resources: ResourceGroup(3155, 2900, 2600, 1205),
                time: 22830,
            },
            ResearchCost {
                resources: ResourceGroup(3970, 3655, 3275, 1515),
                time: 28738,
            },
            ResearchCost {
                resources: ResourceGroup(4745, 4365, 3915, 1810),
                time: 34355,
            },
            ResearchCost {
                resources: ResourceGroup(5495, 5055, 4530, 2095),
                time: 39749,
            },
            ResearchCost {
                resources: ResourceGroup(6215, 5715, 5125, 2370),
                time: 44966,
            },
            ResearchCost {
                resources: ResourceGroup(6915, 6360, 5700, 2640),
                time: 50036,
            },
            ResearchCost {
                resources: ResourceGroup(7595, 6990, 6265, 2900),
                time: 54980,
            },
            ResearchCost {
                resources: ResourceGroup(8265, 7605, 6815, 3155),
                time: 59815,
            },
            ResearchCost {
                resources: ResourceGroup(8920, 8205, 7355, 3405),
                time: 64554,
            },
            ResearchCost {
                resources: ResourceGroup(9565, 8795, 7885, 3650),
                time: 69208,
            },
            ResearchCost {
                resources: ResourceGroup(10195, 9380, 8405, 3890),
                time: 73784,
            },
            ResearchCost {
                resources: ResourceGroup(10820, 9950, 8920, 4130),
                time: 78291,
            },
            ResearchCost {
                resources: ResourceGroup(11435, 10515, 9425, 4365),
                time: 82733,
            },
            ResearchCost {
                resources: ResourceGroup(12040, 11075, 9925, 4595),
                time: 87117,
            },
            ResearchCost {
                resources: ResourceGroup(12635, 11625, 10420, 4825),
                time: 91447,
            },
            ResearchCost {
                resources: ResourceGroup(13230, 12170, 10905, 5050),
                time: 95725,
            },
            ResearchCost {
                resources: ResourceGroup(13815, 12705, 11385, 5270),
                time: 99957,
            },
            ResearchCost {
                resources: ResourceGroup(14390, 13240, 11865, 5495),
                time: 104144,
            },
        ],
    },
    // 6. Haeduan ($ab26)
    SmithyUnitUpgrades {
        unit: UnitName::Haeduan,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1200, 1480, 1640, 450),
                time: 11160,
            },
            ResearchCost {
                resources: ResourceGroup(2090, 2575, 2860, 785),
                time: 19431,
            },
            ResearchCost {
                resources: ResourceGroup(2890, 3565, 3955, 1085),
                time: 26876,
            },
            ResearchCost {
                resources: ResourceGroup(3640, 4485, 4975, 1365),
                time: 33831,
            },
            ResearchCost {
                resources: ResourceGroup(4350, 5365, 5950, 1630),
                time: 40443,
            },
            ResearchCost {
                resources: ResourceGroup(5030, 6205, 6885, 1885),
                time: 46793,
            },
            ResearchCost {
                resources: ResourceGroup(5690, 7020, 7785, 2135),
                time: 52935,
            },
            ResearchCost {
                resources: ResourceGroup(6335, 7810, 8665, 2375),
                time: 58903,
            },
            ResearchCost {
                resources: ResourceGroup(6960, 8585, 9520, 2610),
                time: 64723,
            },
            ResearchCost {
                resources: ResourceGroup(7570, 9340, 10360, 2840),
                time: 70415,
            },
            ResearchCost {
                resources: ResourceGroup(8170, 10080, 11180, 3065),
                time: 75994,
            },
            ResearchCost {
                resources: ResourceGroup(8760, 10805, 11985, 3285),
                time: 81472,
            },
            ResearchCost {
                resources: ResourceGroup(9340, 11520, 12775, 3500),
                time: 86860,
            },
            ResearchCost {
                resources: ResourceGroup(9910, 12225, 13560, 3715),
                time: 92165,
            },
            ResearchCost {
                resources: ResourceGroup(10475, 12915, 14325, 3925),
                time: 97395,
            },
            ResearchCost {
                resources: ResourceGroup(11030, 13600, 15085, 4135),
                time: 102556,
            },
            ResearchCost {
                resources: ResourceGroup(11575, 14275, 15835, 4340),
                time: 107652,
            },
            ResearchCost {
                resources: ResourceGroup(12115, 14945, 16575, 4545),
                time: 112689,
            },
            ResearchCost {
                resources: ResourceGroup(12655, 15605, 17310, 4745),
                time: 117670,
            },
            ResearchCost {
                resources: ResourceGroup(13185, 16260, 18035, 4945),
                time: 122599,
            },
        ],
    },
    // 7. Battering Ram (Gaul) ($ab27)
    SmithyUnitUpgrades {
        unit: UnitName::Ram,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(2250, 1330, 835, 230),
                time: 16800,
            },
            ResearchCost {
                resources: ResourceGroup(3915, 2315, 1455, 400),
                time: 29250,
            },
            ResearchCost {
                resources: ResourceGroup(5420, 3200, 2015, 550),
                time: 40458,
            },
            ResearchCost {
                resources: ResourceGroup(6820, 4025, 2535, 690),
                time: 50928,
            },
            ResearchCost {
                resources: ResourceGroup(8155, 4815, 3030, 825),
                time: 60881,
            },
            ResearchCost {
                resources: ResourceGroup(9435, 5570, 3510, 955),
                time: 70442,
            },
            ResearchCost {
                resources: ResourceGroup(10670, 6300, 3970, 1085),
                time: 79687,
            },
            ResearchCost {
                resources: ResourceGroup(11875, 7010, 4415, 1205),
                time: 88671,
            },
            ResearchCost {
                resources: ResourceGroup(13050, 7705, 4850, 1325),
                time: 97432,
            },
            ResearchCost {
                resources: ResourceGroup(14195, 8380, 5280, 1440),
                time: 106001,
            },
            ResearchCost {
                resources: ResourceGroup(15320, 9045, 5695, 1555),
                time: 114399,
            },
            ResearchCost {
                resources: ResourceGroup(16425, 9695, 6110, 1665),
                time: 122646,
            },
            ResearchCost {
                resources: ResourceGroup(17510, 10340, 6510, 1775),
                time: 130757,
            },
            ResearchCost {
                resources: ResourceGroup(18580, 10970, 6910, 1885),
                time: 138743,
            },
            ResearchCost {
                resources: ResourceGroup(19635, 11595, 7300, 1995),
                time: 146616,
            },
            ResearchCost {
                resources: ResourceGroup(20675, 12205, 7690, 2100),
                time: 154385,
            },
            ResearchCost {
                resources: ResourceGroup(21705, 12815, 8070, 2205),
                time: 162057,
            },
            ResearchCost {
                resources: ResourceGroup(22720, 13415, 8450, 2305),
                time: 169640,
            },
            ResearchCost {
                resources: ResourceGroup(23725, 14005, 8820, 2410),
                time: 177138,
            },
            ResearchCost {
                resources: ResourceGroup(24720, 14595, 9190, 2510),
                time: 184558,
            },
        ],
    },
    // 8. Catapult (Gaul) ($ab28)
    SmithyUnitUpgrades {
        unit: UnitName::Trebuchet,
        costs_per_level: [
            ResearchCost {
                resources: ResourceGroup(1135, 1710, 770, 130),
                time: 28800,
            },
            ResearchCost {
                resources: ResourceGroup(1980, 2975, 1340, 230),
                time: 50144,
            },
            ResearchCost {
                resources: ResourceGroup(2735, 4115, 1850, 315),
                time: 69357,
            },
            ResearchCost {
                resources: ResourceGroup(3445, 5180, 2330, 400),
                time: 87305,
            },
            ResearchCost {
                resources: ResourceGroup(4120, 6190, 2785, 475),
                time: 104368,
            },
            ResearchCost {
                resources: ResourceGroup(4765, 7165, 3220, 550),
                time: 120757,
            },
            ResearchCost {
                resources: ResourceGroup(5390, 8105, 3645, 625),
                time: 136606,
            },
            ResearchCost {
                resources: ResourceGroup(6000, 9015, 4055, 695),
                time: 152007,
            },
            ResearchCost {
                resources: ResourceGroup(6590, 9910, 4455, 765),
                time: 167027,
            },
            ResearchCost {
                resources: ResourceGroup(7170, 10780, 4850, 830),
                time: 181716,
            },
            ResearchCost {
                resources: ResourceGroup(7740, 11635, 5230, 895),
                time: 196113,
            },
            ResearchCost {
                resources: ResourceGroup(8300, 12470, 5610, 960),
                time: 210251,
            },
            ResearchCost {
                resources: ResourceGroup(8845, 13295, 5980, 1025),
                time: 224154,
            },
            ResearchCost {
                resources: ResourceGroup(9385, 14110, 6345, 1085),
                time: 237845,
            },
            ResearchCost {
                resources: ResourceGroup(9920, 14910, 6705, 1150),
                time: 251342,
            },
            ResearchCost {
                resources: ResourceGroup(10445, 15700, 7060, 1210),
                time: 264660,
            },
            ResearchCost {
                resources: ResourceGroup(10965, 16480, 7410, 1270),
                time: 277812,
            },
            ResearchCost {
                resources: ResourceGroup(11480, 17250, 7760, 1330),
                time: 290811,
            },
            ResearchCost {
                resources: ResourceGroup(11985, 18015, 8100, 1390),
                time: 303665,
            },
            ResearchCost {
                resources: ResourceGroup(12485, 18765, 8440, 1445),
                time: 316385,
            },
        ],
    },
];
