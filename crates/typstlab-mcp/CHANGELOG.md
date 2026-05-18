# Changelog

## [0.2.0](https://github.com/SotaTne/typstlab/compare/v0.1.0...v0.2.0) (2026-05-18)


### Features

* **mcp:** implement build tool ([1ce2f3d](https://github.com/SotaTne/typstlab/commit/1ce2f3d0fa2ec52c8e2dbb2511ef6366e4cf3bab))
* **mcp:** implement project root detection ([c579b97](https://github.com/SotaTne/typstlab/commit/c579b971eb61c2127f3d69544c1fcfac81e24f45))
* **mcp:** implement rules resource ([c9959b6](https://github.com/SotaTne/typstlab/commit/c9959b68c5c490dfb6aa18689f2ad2e8a77f66d8))
* **mcp:** remove all rules related tools and resources ([e49bf3d](https://github.com/SotaTne/typstlab/commit/e49bf3d7a5371bee06d1ac3809701d288b65e25e))
* mcpのrmcp全面対応 ([cb68dff](https://github.com/SotaTne/typstlab/commit/cb68dffbf6c094e0f22732cd7867789b75d83453))
* mcpの検索とgetにページング機能の追加 ([7fa7513](https://github.com/SotaTne/typstlab/commit/7fa75131731b56b58af82ff36764e3093f44980f))
* mcpの検索をより型的に安全で一般的な形にした ([13fad7d](https://github.com/SotaTne/typstlab/commit/13fad7d7815280f59d77d6041b42b8a3aa9f26bd))
* mcpの設計およびリファクタリング ([5f0bc23](https://github.com/SotaTne/typstlab/commit/5f0bc23583dc8eaa67417820bc3c3c1ffdf401fd))
* rules mcpの追加 ([9e46bfe](https://github.com/SotaTne/typstlab/commit/9e46bfee3ce3b9947d79864eb7665cbb48fe5264))
* rulesとdocsに特定のpathの中身を取得するgetの追加 ([c995451](https://github.com/SotaTne/typstlab/commit/c99545166bd04dd33bb318dee6f879a261f08fb4))
* **typstlab-mcp:** 画像としてmcpに理解させる機能の追加 ([4ccdaff](https://github.com/SotaTne/typstlab/commit/4ccdaff3e33b780115b92cfdad5754eeccdabcce))
* 細かい仕様変更とその実装 ([8c7e1d3](https://github.com/SotaTne/typstlab/commit/8c7e1d32c87845273228176e0bc9d7cf998233b0))


### Bug Fixes

* copilotでのテストが通るように修正 ([616ce67](https://github.com/SotaTne/typstlab/commit/616ce67bcd84dc8c2f29aa577c7b7e02797f7fa6))
* **core,mcp:** use path abstraction for validation ([8b1dcf1](https://github.com/SotaTne/typstlab/commit/8b1dcf1ee93fc199f3912393a8e97a1e6d91d8a8))
* fmtの実行 ([f13144a](https://github.com/SotaTne/typstlab/commit/f13144a1d76d0baa5127df3f89fe02ac9a4f33e0))
* **mcp:** resolve rules tool import errors ([c8f506d](https://github.com/SotaTne/typstlab/commit/c8f506d1ba82aab49cebb101ba65135c6cbf896b))
* mcpのtypeのobjectへの統一とそのテストの追加 ([547b062](https://github.com/SotaTne/typstlab/commit/547b062514ce553a6aebbe6a7ef2bf0e48646547))
* **typstlab-mcp, typstlab-typst:** security, bounds checking, and robustness fixes ([b766def](https://github.com/SotaTne/typstlab/commit/b766def24dfca37cddc5f03cfaaf41a99c336562))
* **typstlab-mcp:** enforce 1-indexed line numbers for all cases including empty files ([c0436f2](https://github.com/SotaTne/typstlab/commit/c0436f2441018aa81da799f97fe3a4250ddd2d9d))
* **typstlab-mcp:** ensure 1-indexed API consistency for empty files ([0eb8108](https://github.com/SotaTne/typstlab/commit/0eb810842b6d607c62b29e11371c3a9126d599c8))
* **typstlab-mcp:** fix empty file range inversion, reject max_lines=0, and refactor to Path types ([8b74439](https://github.com/SotaTne/typstlab/commit/8b74439f8d51663bafb69f9c13b8193ceebaad79))
* **typstlab-mcp:** use zero-indexed special case for empty files to maintain invariants ([a220836](https://github.com/SotaTne/typstlab/commit/a22083648c6704585bf4606c4a3e1ba93a9e9507))
* 検索がどういう検索かの説明がなかったのでそのような説明に仕様を含めて修正した ([33dd1a3](https://github.com/SotaTne/typstlab/commit/33dd1a316fdfd56a069cae3488ae5abcbe7ea720))
