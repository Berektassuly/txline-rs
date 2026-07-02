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
  process.env.TXLINE_TRADING_GOLDEN_OUT ||
  path.join(__dirname, "trading_golden.devnet.json");

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
      "regenerating trading golden fixtures.",
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

const fixtureSummary = {
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
const terms = {
  fixture_id: new BN("2147483653"),
  period: 0,
  stat_a_key: 1001,
  stat_b_key: 1002,
  predicate: { threshold: 1, comparison: { GreaterThan: {} } },
  op: { Subtract: {} },
  negation: false,
};

const payloads = {
  create_intent: {
    intent_id: new BN("9001"),
    terms_hash: hash(100),
    deposit_amount: new BN("123456789"),
    expiration_ts: new BN("1781129999999"),
    claim_period: 42,
    fixture_id: new BN("2147483653"),
  },
  create_trade: {
    trade_id: new BN("9002"),
    stake_a: new BN("111111"),
    stake_b: new BN("222222"),
    trade_terms_hash: hash(110),
  },
  execute_match: {
    trade_id: new BN("9003"),
    maker_stake: new BN("333333"),
    taker_stake: new BN("444444"),
  },
  close_intent: {},
  settle_trade: {
    trade_id: new BN("9004"),
    ts: new BN("1781123456789"),
    fixture_summary: fixtureSummary,
    fixture_proof: [proof(50, false)],
    main_tree_proof: [proof(60, true)],
    predicate: { threshold: 1, comparison: { LessThan: {} } },
    stat_a: statA,
    stat_b: statB,
    op: { Add: {} },
  },
  settle_matched_trade: {
    trade_id: new BN("9005"),
    ts: new BN("1781123456790"),
    fixture_summary: fixtureSummary,
    fixture_proof: [proof(51, false)],
    main_tree_proof: [proof(61, true)],
    stat_a: statA,
    stat_b: statB,
    terms,
  },
  claim_via_resolution: {
    epoch_day: 20615,
    interval_index: 17,
    merkle_proof: [proof(70, false), proof(71, true)],
  },
  claim_batch_legacy: {
    epoch_day: 20616,
    interval_index: 18,
    terms_hash: hash(120),
    winner_is_maker: true,
    seq: 941,
    merkle_proof: [proof(72, false), proof(73, true)],
  },
  refund_batch: {},
  audit_trade_result: {
    terms: {
      ...terms,
      stat_b_key: null,
      op: null,
      negation: true,
    },
    fixture_summary: fixtureSummary,
    main_tree_proof: [proof(62, true)],
    fixture_proof: [proof(52, false)],
    stat_a: statA,
    stat_b: null,
    ts: new BN("1781123456791"),
  },
};

function dataHex(name, payload) {
  return Buffer.from(coder.encode(name, payload)).toString("hex");
}

const fixtureNames = [
  "create_intent",
  "create_trade",
  "execute_match",
  "close_intent",
  "settle_trade",
  "settle_matched_trade",
  "claim_via_resolution",
  "claim_batch_legacy",
  "refund_batch",
  "audit_trade_result",
];

const output = {
  network: "devnet",
  source: {
    repository: "https://github.com/txodds/tx-on-chain",
    ref: txOnChainRef,
    commit,
    idlPath,
    anchorVersion: anchorPackage.version,
  },
  generatedBy: "crates/txline/tests/fixtures/generate_trading_golden.js",
  fixtures: fixtureNames.map((name) => ({
    name,
    rustBuilder: `${name}_instruction`,
    dataHex: dataHex(name, payloads[name]),
  })),
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
