# Releasing

This repository publishes language packages through a manually triggered GitHub
Actions workflow. The workflow is intentionally manual for first releases: it
keeps registry credentials out of local machines, uses short-lived OIDC tokens
where registries support them, and gives the GitHub `release` environment a
chance to require review before publish jobs start.

## Release Workflow

The publishing workflow is `.github/workflows/release.yml`.

It runs only through `workflow_dispatch` and requires:

- the workflow to be started from `main`,
- `confirm_publish` set to `yes`,
- at least one package version input,
- the GitHub Actions environment named `release`.

Leave a package version input blank to skip that package.

| Input | Publishes | Version source |
| --- | --- | --- |
| `rust_version` | `txline` on crates.io | `crates/txline/Cargo.toml` |
| `python_version` | `txline` on PyPI | `python/pyproject.toml` |
| `npm_version` | `@beriktassuly/txline` on npm | `typescript/package.json` |
| `go_version` | Go module tag `go/vX.Y.Z` | workflow input |

The workflow validates that requested registry versions match checked-in package
metadata before publishing.

## Published SDK Links

| Platform | Package | Link |
| --- | --- | --- |
| Rust | `txline` | <https://crates.io/crates/txline> |
| Python | `txline` | <https://pypi.org/project/txline/> |
| TypeScript | `@beriktassuly/txline` | <https://www.npmjs.com/package/@beriktassuly/txline> |
| Go | `github.com/Berektassuly/txline/go/txline` | <https://pkg.go.dev/github.com/Berektassuly/txline/go/txline> |

## One-Time GitHub Setup

Create a repository environment named `release`:

1. Open `Settings -> Environments`.
2. Create `release`.
3. Add required reviewers for publishing.
4. Keep secrets empty unless a registry explicitly requires a bootstrap token.

The `release` environment name must match the trusted publisher configuration
on registries that use OIDC.

## PyPI

Package: `txline`

PyPI supports pending trusted publishers, so the first Python package release can
be published from CI without a local API token.

Use these values when adding the GitHub publisher on PyPI:

| Field | Value |
| --- | --- |
| PyPI project name | `txline` |
| Owner | `Berektassuly` |
| Repository name | `txline` |
| Workflow filename | `release.yml` |
| Environment name | `release` |

The workflow builds from `python/` and publishes `python/dist` through
`pypa/gh-action-pypi-publish@release/v1` with `id-token: write`.

## crates.io

Package: `txline`

The Rust crate has already had an initial crates.io release, so trusted
publishing can be configured for future versions.

Use these values on the crate trusted publisher settings page:

| Field | Value |
| --- | --- |
| Crate | `txline` |
| Owner | `Berektassuly` |
| Repository name | `txline` |
| Workflow filename | `release.yml` |
| Environment name | `release` |

Before a Rust release, bump `crates/txline/Cargo.toml` to an unpublished version.
The workflow checks the version, runs the Rust tests, performs
`cargo publish --dry-run`, exchanges the GitHub OIDC token for a temporary
crates.io token, and runs `cargo publish -p txline`.

## npm

Package: `@beriktassuly/txline`

The package metadata includes the GitHub repository URL required by npm trusted
publishing. npm trusted publishing requires Node `22.14.0` or newer and npm
`11.5.1` or newer; the workflow uses Node 24 and upgrades npm before publishing.

Use these values when configuring the trusted publisher for the package:

| Field | Value |
| --- | --- |
| Package | `@beriktassuly/txline` |
| Owner | `Berektassuly` |
| Repository name | `txline` |
| Workflow filename | `release.yml` |
| Environment name | `release` |
| Allowed actions | `npm publish` |

If npm requires the package to exist before the trusted publisher can be saved,
do one bootstrap publish with a short-lived automation token, then remove the
token and configure trusted publishing for later releases. After that, the
workflow publishes with OIDC and no long-lived npm token.

## Go Module

Package path: `github.com/Berektassuly/txline/go`

Go modules do not use a registry publish API. Publishing is a Git tag operation.
Because the module root is the `go/` subdirectory, release tags must use the
subdirectory prefix:

```text
go/v0.4.0
```

The workflow accepts `go_version` as `0.4.0` or `v0.4.0`, runs the same Go
quality gates as CI, creates and pushes the annotated tag, verifies the pushed
subdirectory module through `GOPROXY=direct`, and then best-effort warms the
public Go proxy.

## First Release Sequence

1. Merge the package PR into `main`.
2. Create the GitHub `release` environment.
3. Add the PyPI pending trusted publisher for `txline`.
4. Add the crates.io trusted publisher for `txline`.
5. Configure npm trusted publishing for `@beriktassuly/txline`, or do the npm bootstrap
   publish if the package page must exist first.
6. Open `Actions -> Release -> Run workflow` from `main`.
7. Set `confirm_publish` to `yes`.
8. Fill only the package versions you want to publish.

For the first package release, prefer publishing one ecosystem at a time. That
keeps failures isolated while the registry-side trusted publisher settings are
still new.

## References

- PyPI Trusted Publishers:
  <https://docs.pypi.org/trusted-publishers/using-a-publisher/>
- PyPI pending publishers:
  <https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/>
- crates.io trusted publishing:
  <https://crates.io/docs/trusted-publishing>
- npm trusted publishing:
  <https://docs.npmjs.com/trusted-publishers/>
- Go module version tags:
  <https://go.dev/ref/mod#vcs-version>
