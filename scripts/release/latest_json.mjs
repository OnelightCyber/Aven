// Writes the manifest Aven's updater reads from a release, to standard output.
//
//   node scripts/release/latest_json.mjs <version> <owner/repo> <installer.sig> [notes]
//
// The installer is published as Aven-Windows-x64-setup.exe on the release
// tagged v<version>. Its signature goes in the manifest as the signer wrote
// it; the updater checks it against the key built into the app, so a
// manifest that is tampered with only makes an update fail.
import fs from "node:fs";

const INSTALLER = "Aven-Windows-x64-setup.exe";

export function manifest(version, repository, signature, notes, published = new Date()) {
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error("the version is not a release version");
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) throw new Error("the repository is not owner/name");
  const signed = signature.trim();
  if (!/^[A-Za-z0-9+/=]{100,}$/.test(signed)) throw new Error("the signature is not what the signer writes");
  const entry = {
    signature: signed,
    url: `https://github.com/${repository}/releases/download/v${version}/${INSTALLER}`,
  };
  return {
    version,
    notes: notes?.trim() || `Aven ${version}`,
    pub_date: published.toISOString(),
    // The updater asks for the installer kind first, then the plain target.
    platforms: { "windows-x86_64-nsis": entry, "windows-x86_64": entry },
  };
}

if (process.argv[1]?.replaceAll("\\", "/").endsWith("scripts/release/latest_json.mjs")) {
  const [version, repository, signaturePath, notes] = process.argv.slice(2);
  if (!version || !repository || !signaturePath) {
    console.error("usage: node scripts/release/latest_json.mjs <version> <owner/repo> <installer.sig> [notes]");
    process.exit(2);
  }
  try {
    const signature = fs.readFileSync(signaturePath, "utf8");
    process.stdout.write(`${JSON.stringify(manifest(version, repository, signature, notes), null, 2)}\n`);
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
