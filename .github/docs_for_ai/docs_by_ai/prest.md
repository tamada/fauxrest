# `prest` 仕様書: 規約駆動の静的 API ジェネレータ

## 1. 思想 (Philosophy)

`prest` は、Ruby on Rails の「**設定より規約 (Convention over Configuration)**」の思想に強く影響を受けています。開発者は詳細な設定を記述することなく、直感的な規約に従うだけで、堅牢な静的 RESTful API を構築できます。

その中核をなすのが「**データソース駆動 (Data-Source Driven)**」というアプローチです。APIの構造は、設定ファイルではなく、`data/` ディレクトリに配置された JSON データそのものによって定義されます。

設定ファイルは、規約から外れたい例外的なケースや、プロジェクト全体の出力形式をカスタマイズするために、**オプション**として存在します。

## 2. グローバル設定 (Global Configuration)

プロジェクトのルートに配置する設定ファイル（例: `prest.json`）のトップレベルに `$config` キーを設けることで、API全体の出力形式を制御できます。これらの設定はオプションであり、記述がない場合はデフォルト値が適用されます。

```json
// prest.json の例
{
  "$config": {
    "collectionAsDirectory": true,
    "resourceHasExtension": false
  },
  "api": {
    // ... APIの構造定義 ...
  }
}
```

### 設定項目

-   **`collectionAsDirectory`** (真偽値)
    -  **`true` (デフォルト)**: Collection API（一覧）を `api/papers/index.json` のように、ディレクトリ内の `index.json`として生成します。これにより、クライアントは `/api/papers` というクリーンなURLでアクセスできます。
    -  **`false`**: Collection API を `api/papers.json` のように、拡張子付きの単一ファイルとして生成します。

-   **`resourceHasExtension`** (真偽値)
    -   **`false` (デフォルト)**: 個別のリソースを `api/profile` のように、拡張子なしのファイルとして生成します。
    -   **`true`**: 個別のリソースを `api/profile.json` のように、拡張子付きのファイルとして生成します。

## 3. 基本規約 (Core Conventions)

ツールは、以下の規約に基づいて API の構造を自動的に推論し、静的ファイルを生成します。

#### 規約1: データソースが `Collection` API になる

`data/` ディレクトリに配置された各JSONファイルは、リソースの**一覧 (Collection)** を返す API エンドポイントに直接対応します。

-   **もし** `data/papers.json` が存在する **ならば**
-   **ツールは** `GET /api/papers` というエンドポイントを自動生成する。

#### 規約2: `id` フィールドが `Individual Resource` API になる

Collection のデータが、`"id"` キーを持つオブジェクトの配列で構成されている場合、ツールは個別のリソースを取得するエンドポイントも自動で生成します。

-   **もし** `data/papers.json` の各要素が `{"id": "...", ...}` という構造を持つ **ならば**
-   **ツールは** `GET /api/papers/{id}` というエンドポイントを自動生成する。

#### 規約3: APIのルートは「目次」になる

APIの最上位階層 (ルート) は、API全体で利用可能なエンドポイントの一覧、すなわち**「目次 (Discovery Document)」**を返します。

-   **デフォルトで** `GET /api` は、API全体の目次を返すエンドポイントとなる。

## 4. 設定による規約の上書きと拡張 (Configuration for Exceptions)

規約だけでは表現できないエンドポイントを定義するために、設定ファイルを利用できます。

#### メタキー `$index` の定義

-   `"$index": "discovery"`
    -   **意味**: その階層の `index.json` が**「目次」**の役割を持つことを明示します。

-   `"$index": "collection"`
    -   **意味**: その階層の `index.json` が**「データ一覧」**の役割を持つことを明示します。

-   `"$index": { "aggregate": ["/path1", ...] }`
    -   **意味**: 複数の Collection データソースを**集約 (Aggregate)** した、単一の `index.json` を生成します。

-   `"$index": false`
    -   **意味**: 規約による `index.json` の自動生成を**明示的に抑制**します。

## 5. 物理ファイル・マッピングの最終ルール

以下の表は、グローバル設定の組み合わせによって、APIエンドポイントがどのように物理ファイルパスに対応するかを示します。

| API エンドポイント | 生成条件 | デフォルト設定でのパス<br>(`collectionAsDirectory: true`, `resourceHasExtension: false`) | `collectionAsDirectory: false` | `resourceHasExtension: true` |
| :--- | :--- | :--- | :--- | :--- |
| `GET /api` | **規約** | `api/index.json` | `api.json` | `api/index.json` |
| `GET /api/papers` | `data/papers.json` が存在 | `api/papers/index.json` | `api/papers.json` | `api/papers/index.json` |
| `GET /api/papers/{id}` | `data/papers.json` に `id` が存在 | `api/papers/{id}` | `api/papers/{id}` | `api/papers/{id}.json` |
| `GET /api/profile` | 設定ファイルで単体リソースとして定義 | `api/profile` | `api/profile` | `api/profile.json` |
| `GET /api/all` | 設定で `aggregate` を定義 | `api/all/index.json` | `api/all.json` | `api/all/index.json` |
