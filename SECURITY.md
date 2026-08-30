# Security Policy

## Supported versions

StreamShougiBoard は、原則として GitHub Releases の最新バージョンと `main` ブランチを対象に
セキュリティ修正を行います。古い配布バイナリで問題が発生した場合は、まず最新リリースで
再現するか確認してください。

## Reporting a vulnerability

脆弱性の可能性がある問題は、公開 Issue へ詳細を書かず、リポジトリの
**Security → Advisories → Report a vulnerability** から非公開で報告してください。

報告には、可能な範囲で次を含めてください。

- 影響を受ける StreamShougiBoard のバージョン
- Windows と OBS Studio のバージョン
- 再現手順と想定される影響
- 検証に使ったコードやログ（秘密情報は除去してください）
- 公開時に希望するクレジット表記

受領後は原則 7 日以内を目安に初回回答し、影響確認、修正、リリース、公開時期を報告者と
調整します。実際の対応期間は影響範囲と修正難度によって変わります。

StreamShougiBoard の脅威モデルと実装上の防御は
[docs/security.md](docs/security.md) を参照してください。
