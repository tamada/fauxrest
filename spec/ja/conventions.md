# prest 基本規約と物理ファイルマッピング仕様 (Conventions)

`prest` は、詳細な設定ファイルの記述を必要とせず、あらかじめ決められた直感的な規約に従うことで、堅牢な静的 RESTful API を高速に構築できる**規約駆動型（Convention over Configuration）**の静的 API ジェネレータです。

本書では、その根幹となる思想、グローバル設定、基本規約、および複数のシリアライザーを並行して実行可能なマルチ・シリアライザー・アーキテクチャについて定義します。

---

## 1. 設計思想 (Philosophy)

- **設定より規約 (Convention over Configuration)**: 
  開発者が明示的なルート定義をすることなく、データフォルダ（`data/`）の構造を整えるだけで、自動的に最適な API を構築できるようにします。
- **データソース駆動 (Data-Source Driven)**: 
  API のエンドポイントやツリー構造は、配置された JSON データそのものの構造によって自動定義されます。設定ファイルは規約から外れたい例外やカスタマイズのための「オプション」として機能します。

---

## 2. マルチ・シリアライザー構成 ($config.serializers)

プロジェクトの用途が多岐にわたる本番運用において、`prest` は1回のビルドで複数のデータ成果物（Web 配信用の JSON、フロントエンドビルド用の JS/TS モジュール、オフラインクライアント用の SQLite データベース）を同時にコンパイルして出力できる **「マルチ・シリアライザー（Multi-Serializer）」** アーキテクチャを採用しています。

プロジェクトルートの設定ファイル（例: `prest.json`）の `$config.serializers` リストの中で、シリアライザーごとに独立した「配信レイアウト（`layout`）」と「出力先（`dest`）」を定義できます。

```json
{
  "$config": {
    "serializers": [
      {
        "serializer": "json",
        "layout": "index",
        "dest": "./dist/api"
      },
      {
        "serializer": "typescript",
        "layout": "file",
        "dest": "./dist/modules"
      },
      {
        "serializer": "sqlite",
        "dest": "./dist/db/api.db"
      }
    ]
  }
}
```

### 各設定パラメータの役割

1. **`serializer` (物理フォーマット)**
   - **値**: `"json"` | `"typescript"` (または `"js"`) | `"sqlite"`
   - データの物理的な書き出し形式を定義します。
2. **`layout` (配置・配信レイアウト - オプショナル)**
   - **値**: `"index"` | `"file"` | `"extension"`
   - 物理ファイルを、配信サーバーの特性に合わせたどのようなフォルダ・ファイル構造（レイアウト）でコンパイルするかを定義します。
   - ※ `sqlite` シリアライザーのように、成果物が単一のバイナリファイルとなる場合は `layout` パラメータは不要です。
3. **`dest` (出力先パス)**
   - 各シリアライザーが成果物を書き出す独立したディレクトリ（または SQLite ファイル名）を指定します。
   - 出力先を分離できるため、同じ `file`（拡張子なし）レイアウトを採用していても、ファイルの上書き（衝突）が完全に回避されます。

---

## 3. 配信レイアウト（Layout）の定義

`layout` パラメータは、静的ファイルサーバーにおける配信特性に基づき、物理ファイルの配置構造を決定します。

1. **`index` レイアウト**
   - **内容**: コレクションも個別リソースも、すべてのエンドポイントをディレクトリ化し、内部に `index.json`（または `index.ts` 等）を出力。
   - **メリット**: クリーン URL を維持しつつ、すべてのホスティングサーバー（S3, GitHub Pages 等）で完璧に動作。

2. **`file` レイアウト (スマート・フォールバック内蔵)**
   - **内容**: すべてのエンドポイントを、拡張子なしのプレーンなファイルとして直接出力。
   - **メリット**: 拡張子なしファイルに対して正しい MIME タイプヘッダー（`application/json` や `application/javascript`）を自動付与して配信できる、近代的な CDN（Netlify, Cloudflare Pages 等）での運用に最適。
   - **衝突回避設計**: ファイルシステム上の物理制約を回避するため、配下に個別リソース（サブパス）が存在する箇所には、自動的に `index` 形式へ置き換える**スマート・フォールバック仕様**が組み込まれています。

3. **`extension` レイアウト**
   - **内容**: すべてのエンドポイントに直接拡張子（`.json` や `.ts`）を付与したファイルとして出力。
   - **メリット**: 100% すべての Web サーバー、ローカルファイルシステム上で、一切の設定なしに確実に動作。

---

## 4. 基本規約 (Core Conventions)

データ構造から自動的に API エンドポイントを推論・構築する際の基本ルールです。

- **規約1**: `data/papers.json` が存在 ➔ `GET /api/papers` (コレクション API)
- **規約2**: `id` フィールドが存在 ➔ `GET /api/papers/{id}` (個別リソース API)
- **規約3**: API の最上位階層 ➔ `GET /api` (目次 API / Discovery Document)

---

## 5. 配信レイアウト別の物理ファイル・マッピング表

定義された各シリアライザー（JSON / TS）およびレイアウト（`layout`）が、どのような物理パスへ出力されるかの最終決定ルールです。

### A. `json` シリアライザーでのマッピング（出力先: `./dist/api` と仮定）

| API エンドポイント | `index` レイアウト | `file` レイアウト (★スマート・フォールバック) | `extension` レイアウト |
| :--- | :--- | :--- | :--- |
| **`GET /api`** (ルート目次) | `api/index.json` | **`api/index.json`** *(※配下に papers があるため)* | `api.json` |
| **`GET /api/papers`** (一覧) | `api/papers/index.json` | **`api/papers/index.json`** *(※配下に `{id}` があるため)* | `api/papers.json` |
| **`GET /api/papers/{id}`** (個別) | `api/papers/{id}/index.json` | **`api/papers/{id}`** *(末端のため拡張子なしファイル)* | `api/papers/{id}.json` |
| **`GET /api/profile`** (単一) | `api/profile/index.json` | **`api/profile`** *(配下がないため拡張子なしファイル)* | `api/profile.json` |

### B. `typescript` シリアライザーでのマッピング（出力先: `./dist/modules` と仮定）
※ シリアライズ形式が `.ts` のコード（ESM 形式）に変化します。

| API エンドポイント | `index` レイアウト | `file` レイアウト (★スマート・フォールバック) | `extension` レイアウト |
| :--- | :--- | :--- | :--- |
| **`GET /api`** (ルート目次) | `modules/index.ts` | **`modules/index.ts`** | `modules.ts` |
| **`GET /api/papers`** (一覧) | `modules/papers/index.ts` | **`modules/papers/index.ts`** | `modules/papers.ts` |
| **`GET /api/papers/{id}`** (個別) | `modules/papers/{id}/index.ts` | **`modules/papers/{id}`** | `modules/papers/{id}.ts` |
| **`GET /api/profile`** (単一) | `modules/profile/index.ts` | **`modules/profile`** | `modules/profile.ts` |
