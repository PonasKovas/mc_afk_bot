// maps mob names to their IDs and vice versa

use bimap::BiBTreeMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MOBS: BiBTreeMap<i64, &'static str> = {
        let mut map = BiBTreeMap::new();

        map.insert(3, "Bat");
        map.insert(4, "Bee");
        map.insert(5, "Blaze");
        map.insert(7, "Cat");
        map.insert(8, "Cave Spider");
        map.insert(9, "Chicken");
        map.insert(10, "Cod");
        map.insert(11, "Cow");
        map.insert(12, "Creeper");
        map.insert(13, "Dolphin");
        map.insert(14, "Donkey");
        map.insert(16, "Drowned");
        map.insert(17, "Elder Guardian");
        map.insert(19, "Ender Dragon");
        map.insert(20, "Enderman");
        map.insert(21, "Endermite");
        map.insert(23, "Evoker Fangs");
        map.insert(28, "Fox");
        map.insert(29, "Ghast");
        map.insert(30, "Giant");
        map.insert(31, "Guardian");
        map.insert(32, "Hoglin");
        map.insert(33, "Horse");
        map.insert(34, "Husk");
        map.insert(35, "Illusioner");
        map.insert(42, "Llama");
        map.insert(44, "Magma Cube");
        map.insert(52, "Mule");
        map.insert(53, "Mushroom");
        map.insert(54, "Ocelot");
        map.insert(56, "Panda");
        map.insert(57, "Parrot");
        map.insert(58, "Phantom");
        map.insert(59, "Pig");
        map.insert(60, "Piglin");
        map.insert(61, "Piglin Brute");
        map.insert(62, "Pillager");
        map.insert(63, "Polar Bear");
        map.insert(65, "Pufferfish");
        map.insert(66, "Rabbit");
        map.insert(67, "Ravager");
        map.insert(68, "Salmon");
        map.insert(69, "Sheep");
        map.insert(70, "Shulker");
        map.insert(72, "Silverfish");
        map.insert(73, "Skeleton");
        map.insert(74, "Skeleton Horse");
        map.insert(75, "Slime");
        map.insert(77, "Snow Golem");
        map.insert(80, "Spider");
        map.insert(81, "Squid");
        map.insert(82, "Stray");
        map.insert(83, "Strider");
        map.insert(89, "Trader Llama");
        map.insert(90, "Tropical Fish");
        map.insert(91, "Turtle");
        map.insert(92, "Vex");
        map.insert(93, "Villager");
        map.insert(94, "Vindicator");
        map.insert(95, "Wandering Trader");
        map.insert(96, "Witch");
        map.insert(97, "Wither");
        map.insert(98, "Wither Skeleton");
        map.insert(100, "Wolf");
        map.insert(101, "Zoglin");
        map.insert(102, "Zombie");
        map.insert(103, "Zombie Horse");
        map.insert(104, "Zombie Villager");
        map.insert(105, "Zombified Piglin");

        map
    };
}
