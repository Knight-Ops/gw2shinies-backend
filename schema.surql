-- gw2shinies schema

-- TABLE: item
DEFINE TABLE item SCHEMALESS;
-- Fields (examples, schema is flexible but good to define knowns)
DEFINE FIELD name ON TABLE item TYPE string;
DEFINE FIELD description ON TABLE item TYPE string;
DEFINE FIELD rarity ON TABLE item TYPE string;
DEFINE FIELD level ON TABLE item TYPE int;
DEFINE FIELD vendor_value ON TABLE item TYPE int;
DEFINE FIELD chat_link ON TABLE item TYPE string;
DEFINE FIELD icon ON TABLE item TYPE string;
DEFINE FIELD type ON TABLE item TYPE string;
DEFINE FIELD default_skin ON TABLE item TYPE option<int>;
DEFINE FIELD game_types ON TABLE item TYPE array;
DEFINE FIELD flags ON TABLE item TYPE array;
DEFINE FIELD restrictions ON TABLE item TYPE array;
DEFINE FIELD details ON TABLE item TYPE option<object>;
DEFINE FIELD upgrades_into ON TABLE item TYPE option<array>;
DEFINE FIELD upgrades_from ON TABLE item TYPE option<array>;
DEFINE FIELD last_updated ON TABLE item TYPE datetime DEFAULT time::now();

-- TABLE: recipe
DEFINE TABLE recipe SCHEMALESS;
DEFINE FIELD output_item_id ON TABLE recipe TYPE int;
DEFINE FIELD output_item_count ON TABLE recipe TYPE int;
DEFINE FIELD min_rating ON TABLE recipe TYPE int;
DEFINE FIELD time_to_craft_ms ON TABLE recipe TYPE int;
DEFINE FIELD disciplines ON TABLE recipe TYPE array;

-- RELATION: consumes (Recipe -> Item with quantity)
DEFINE TABLE consumes SCHEMALESS;
DEFINE FIELD quantity ON TABLE consumes TYPE int;

-- RELATION: produces (Recipe -> Item with quantity)
-- Usually recipe has one output, but relation is good for graph traversal
DEFINE TABLE produces SCHEMALESS;
DEFINE FIELD quantity ON TABLE produces TYPE int;

-- INDEXES
DEFINE INDEX item_name_idx ON TABLE item COLUMNS name SEARCH ANALYZER ascii BM25 HIGHLIGHTS;
DEFINE INDEX item_history_item_ts_idx ON TABLE item_history COLUMNS item, timestamp;
