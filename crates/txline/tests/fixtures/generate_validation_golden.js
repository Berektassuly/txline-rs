#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { createRequire } = require("module");
const { execFileSync } = require("child_process");

const PINNED_TX_ON_CHAIN_REF = "8dfc6608252f4034a0279b48578c8fe07b949af0";
const ANCHOR_PACKAGE_NAME = "@coral-xyz/anchor";
const EXPECTED_ANCHOR_VERSION = "0.32.1";

const checkOnly = process.argv.includes("--check");
const txOnChainRoot = process.env.TX_ON_CHAIN_ROOT;
const txOnChainRef = process.env.TX_ON_CHAIN_REF || PINNED_TX_ON_CHAIN_REF;
const outputPath =
  process.env.TXLINE_VALIDATION_GOLDEN_OUT ||
  path.join(__dirname, "validation_golden.devnet.json");

if (!txOnChainRoot) {
  throw new Error("TX_ON_CHAIN_ROOT must point to a txodds/tx-on-chain checkout");
}

const requireFromTxOnChain = createRequire(path.join(txOnChainRoot, "package.json"));
const anchor = requireFromTxOnChain(ANCHOR_PACKAGE_NAME);
const anchorPackage = JSON.parse(
  fs.readFileSync(
    path.join(txOnChainRoot, "node_modules", "@coral-xyz", "anchor", "package.json"),
    "utf8",
  ),
);
if (anchorPackage.version !== EXPECTED_ANCHOR_VERSION) {
  throw new Error(
    `${ANCHOR_PACKAGE_NAME} ${anchorPackage.version} found in TX_ON_CHAIN_ROOT; expected ` +
      `${EXPECTED_ANCHOR_VERSION}. Install/update dependencies in the upstream checkout before ` +
      "regenerating validation golden fixtures.",
  );
}

const commit = execFileSync("git", ["-C", txOnChainRoot, "rev-parse", txOnChainRef], {
  encoding: "utf8",
}).trim();
const idlPath = "examples/devnet/idl/txoracle.json";
const idl = JSON.parse(
  execFileSync("git", ["-C", txOnChainRoot, "show", `${txOnChainRef}:${idlPath}`], {
    encoding: "utf8",
  }),
);
const coder = new anchor.BorshInstructionCoder(idl);
const BN = anchor.BN;

function hash(base) {
  return Array.from({ length: 32 }, (_, index) => (base + index) & 0xff);
}

function proof(base, isRightSibling) {
  return {
    hash: hash(base),
    is_right_sibling: isRightSibling,
  };
}

function updateStats(updateCount, minTimestamp, maxTimestamp) {
  return {
    update_count: updateCount,
    min_timestamp: new BN(minTimestamp),
    max_timestamp: new BN(maxTimestamp),
  };
}

const scoreFixtureSummary = {
  fixture_id: new BN("2147483653"),
  update_stats: updateStats(-3, "1781123456789", "1781123456799"),
  events_sub_tree_root: hash(10),
};
const statA = {
  stat_to_prove: { key: 1001, value: 2, period: 0 },
  event_stat_root: hash(20),
  stat_proof: [proof(30, true)],
};
const statB = {
  stat_to_prove: { key: 1002, value: -1, period: 1 },
  event_stat_root: hash(20),
  stat_proof: [proof(40, false)],
};
const validateStat = {
  ts: new BN("1781123456789"),
  fixture_summary: scoreFixtureSummary,
  fixture_proof: [proof(50, false)],
  main_tree_proof: [proof(60, true)],
  predicate: { threshold: 1, comparison: { LessThan: {} } },
  stat_a: statA,
  stat_b: statB,
  op: { Add: {} },
};

const validateStatV2 = {
  payload: {
    ts: new BN("1781123456789"),
    fixture_summary: scoreFixtureSummary,
    fixture_proof: [proof(51, false)],
    main_tree_proof: [proof(61, true)],
    event_stat_root: hash(22),
    stats: [
      {
        stat: { key: 1001, value: 2, period: 0 },
        stat_proof: [proof(31, true)],
      },
      {
        stat: { key: 1002, value: -1, period: 1 },
        stat_proof: [proof(41, false)],
      },
    ],
  },
  strategy: {
    geometric_targets: [
      { stat_index: 0, prediction: 0 },
      { stat_index: 1, prediction: 1 },
    ],
    distance_predicate: { threshold: 2, comparison: { LessThan: {} } },
    discrete_predicates: [
      { Single: { index: 0, predicate: { threshold: 1, comparison: { EqualTo: {} } } } },
      {
        Binary: {
          index_a: 0,
          index_b: 1,
          op: { Subtract: {} },
          predicate: { threshold: 0, comparison: { GreaterThan: {} } },
        },
      },
    ],
  },
};

const fixtureSnapshot = {
  ts: new BN("1781123000000"),
  start_time: new BN("1781126600000"),
  competition: "Devnet Cup",
  competition_id: 7,
  fixture_group_id: -8,
  participant1_id: 101,
  participant1: "Alpha",
  participant2_id: 202,
  participant2: "Beta",
  fixture_id: new BN("2147483654"),
  participant1_is_home: true,
};
const fixtureSummary = {
  fixture_id: new BN("2147483654"),
  competition_id: 7,
  competition: "Devnet Cup",
  update_stats: updateStats(4, "1781123000000", "1781123000001"),
  update_sub_tree_root: hash(70),
};
const validateFixture = {
  snapshot: fixtureSnapshot,
  summary: fixtureSummary,
  sub_tree_proof: [proof(71, false)],
  main_tree_proof: [proof(72, true)],
};

const validateFixtureBatch = {
  index: 3,
  metadata: {
    total_update_count: 5,
    num_unique_fixtures: 2,
    overall_batch_start_ts: new BN("1781123000000"),
    overall_batch_end_ts: new BN("1781123900000"),
  },
  proof: [proof(80, false), proof(81, true)],
};

const odds = {
  fixture_id: new BN("2147483655"),
  message_id: "msg-1",
  ts: new BN("1781123456789"),
  bookmaker: "Book",
  bookmaker_id: 9,
  super_odds_type: "Winner",
  game_state: "PreMatch",
  in_running: false,
  market_parameters: null,
  market_period: "FT",
  price_names: ["Home", "Away"],
  prices: [120, -125],
};
const oddsSummary = {
  fixture_id: new BN("2147483655"),
  update_stats: updateStats(5, "1781123450000", "1781123459999"),
  odds_sub_tree_root: hash(90),
};
const validateOdds = {
  ts: odds.ts,
  odds_snapshot: odds,
  summary: oddsSummary,
  sub_tree_proof: [proof(91, false)],
  main_tree_proof: [proof(92, true)],
};

function dataHex(name, payload) {
  return Buffer.from(coder.encode(name, payload)).toString("hex");
}

const output = {
  network: "devnet",
  source: {
    repository: "https://github.com/txodds/tx-on-chain",
    ref: txOnChainRef,
    commit,
    idlPath,
    anchorVersion: anchorPackage.version,
  },
  generatedBy: "crates/txline/tests/fixtures/generate_validation_golden.js",
  fixtures: [
    {
      name: "validate_stat",
      rustBuilder: "validate_stat_instruction",
      dataHex: dataHex("validate_stat", validateStat),
    },
    {
      name: "validate_stat_v2",
      rustBuilder: "validate_stat_v2_instruction",
      dataHex: dataHex("validate_stat_v2", validateStatV2),
    },
    {
      name: "validate_fixture",
      rustBuilder: "validate_fixture_instruction",
      dataHex: dataHex("validate_fixture", validateFixture),
    },
    {
      name: "validate_fixture_batch",
      rustBuilder: "validate_fixture_batch_instruction",
      dataHex: dataHex("validate_fixture_batch", validateFixtureBatch),
    },
    {
      name: "validate_odds",
      rustBuilder: "validate_odds_instruction",
      dataHex: dataHex("validate_odds", validateOdds),
    },
  ],
};

const serialized = `${JSON.stringify(output, null, 2)}\n`;
if (checkOnly) {
  const existing = fs.readFileSync(outputPath, "utf8");
  if (existing !== serialized) {
    throw new Error(`${outputPath} is out of date; rerun this generator without --check`);
  }
  console.log(`${outputPath} is up to date`);
} else {
  fs.writeFileSync(outputPath, serialized);
  console.log(`wrote ${outputPath}`);
}
