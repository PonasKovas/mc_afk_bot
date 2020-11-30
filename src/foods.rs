// maps food item names to their IDs and vice versa

use bimap::BiBTreeMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref FOODS: BiBTreeMap<i64, &'static str> = {
        let mut map = BiBTreeMap::new();

        map.insert(573, "Apple");
        map.insert(832, "Baked Potato");
        map.insert(889, "Beetroot");
        map.insert(891, "Beetroot Soup");
        map.insert(621, "Bread");
        map.insert(830, "Carrot");
        map.insert(887, "Chorus Fruit");
        map.insert(742, "Cooked Chicken");
        map.insert(691, "Cooked Cod");
        map.insert(869, "Cooked Mutton");
        map.insert(648, "Cooked Porkchop");
        map.insert(856, "Cooked Rabbit");
        map.insert(692, "Cooked Salmon");
        map.insert(732, "Cookie");
        map.insert(736, "Dried Kelp");
        map.insert(651, "Enchanted Golden Apple");
        map.insert(650, "Golden Apple");
        map.insert(835, "Golden Carrot");
        map.insert(955, "Honey Bottle");
        map.insert(735, "Melon Slice");
        map.insert(615, "Mushroom Stew");
        map.insert(833, "Poisonous Potato");
        map.insert(831, "Potato");
        map.insert(690, "Pufferfish");
        map.insert(845, "Pumpkin Pie");
        map.insert(857, "Rabbit Stew");
        map.insert(739, "Raw Beef");
        map.insert(741, "Raw Chicken");
        map.insert(687, "Raw Cod");
        map.insert(868, "Raw Mutton");
        map.insert(647, "Raw Porkchop");
        map.insert(688, "Raw Salmon");
        map.insert(743, "Rotten Flesh");
        map.insert(751, "Spider Eye");
        map.insert(740, "Steak");
        map.insert(927, "Suspicious Stew");
        map.insert(948, "Sweet Berries");
        map.insert(689, "Tropical Fish");

        map
    };
}
