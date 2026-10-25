# macOS distribution signing

Public macOS downloads use a Developer ID Application certificate, Hardened Runtime, a secure timestamp, Apple notarization, and a stapled notarization ticket. The local `npm run build:mac` command intentionally remains suitable for ad-hoc-signed development builds.

## Release procedure

1. Build and test the app, then copy it to a separate distribution staging folder. Preserve the original artifact.
2. Include the MIT notice in `RustyCrone.app/Contents/Resources/LICENSE` before signing. Keep a LICENSE alongside the app in the download as well.
3. Sign `Contents/MacOS/rustycrone` and then the app bundle with `codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY"`. Use a valid Developer ID Application identity from the login Keychain. Do not export the private key or commit credentials.
4. Verify with `codesign --verify --deep --strict` and inspect `codesign -dv --verbose=4` for the Developer ID chain, runtime flag, and secure timestamp.
5. Create a ZIP with `ditto -c -k --sequesterRsrc --keepParent RustyCrone.app submission.zip`. Submit with `xcrun notarytool submit submission.zip --keychain-profile "$NOTARY_PROFILE" --wait`. Require an Accepted result; inspect Apple's log if rejected.
6. Staple the app with `xcrun stapler staple RustyCrone.app`. Validate with `xcrun stapler validate`, strict/deep codesign verification, and `spctl --assess --type execute --verbose=4`. Require `source=Notarized Developer ID`.
7. Package this stapled app without rebuilding or modifying it. For a DMG, include an Applications shortcut and LICENSE. Sign, notarize, and staple the DMG too.
8. Mount the final DMG read-only; repeat app verification against the mounted copy and check a real read/preview workflow. Extract and validate the final ZIP separately.
9. Preserve the prior download, update the release checksums and notes, upload, then compare public asset digests and sizes against the local files. Keep the source tag tied to the app's actual code revision.

`SIGNING_IDENTITY` identifies the local certificate; `NOTARY_PROFILE` names credentials already stored in Keychain. Neither requires putting passwords in a script or source file.

## Certificate-chain errors

An `unable to build chain to self-signed root` error can result from a custom trust override on an Apple code-signing certificate. Apple-issued signing certificates normally use the system's default trust settings. Inspect the exact certificate and its issuers, back up existing trust settings, and restore defaults only for a confirmed problematic override. Do not bypass validation or mark arbitrary certificates as trusted.

Notarization does not grant administrator privileges or Full Disk Access. Those remain separate system permissions.
