# Third-party software

RAGite itself is dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE). This file lists the third-party software it contains or runs, with the notices their licenses ask for.

Cargo crate licenses below are taken from `cargo metadata` for the versions in `Cargo.lock`, or from the crate's LICENSE file when the metadata field is empty. Nothing below is guessed.

## Compiled in or bundled

### SQLite headers 3.50.2

Public domain. The headers `third_party/sqlite/sqlite3.h` and `third_party/sqlite/sqlite3ext.h` are from the SQLite amalgamation `sqlite-amalgamation-3500200` (https://www.sqlite.org/2025/sqlite-amalgamation-3500200.zip). The header begins:

> The author disclaims copyright to this source code. In place of a legal notice, here is a blessing:
>
> May you do good and not evil.
> May you find forgiveness for yourself and forgive others.
> May you share freely, never taking more than you give.

The extension does not compile or link `sqlite3.c`. A host application supplies SQLite and loads RAGite.

### sqlite-vec 0.1.9

MIT OR Apache-2.0. Source commit `e9f598abfa0c06b328d8fe5da9c3760cce74be10` (tag `v0.1.9`, https://github.com/asg017/sqlite-vec). The files `third_party/sqlite-vec/sqlite-vec.c`, `sqlite-vec.h`, `LICENSE-MIT`, and `LICENSE-APACHE` are that revision. `sqlite-vec.h` is generated from the upstream header template with version `0.1.9`, date `2026-03-31`, and that commit. That revision ships no NOTICE file.

MIT license text (`third_party/sqlite-vec/LICENSE-MIT`):

```
MIT License

Copyright (c) 2024 Alex Garcia

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

Apache-2.0 license text (`third_party/sqlite-vec/LICENSE-APACHE`):

```
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1.  Definitions.

    "License" shall mean the terms and conditions for use, reproduction,
    and distribution as defined by Sections 1 through 9 of this document.

    "Licensor" shall mean the copyright owner or entity authorized by
    the copyright owner that is granting the License.

    "Legal Entity" shall mean the union of the acting entity and all
    other entities that control, are controlled by, or are under common
    control with that entity. For the purposes of this definition,
    "control" means (i) the power, direct or indirect, to cause the
    direction or management of such entity, whether by contract or
    otherwise, or (ii) ownership of fifty percent (50%) or more of the
    outstanding shares, or (iii) beneficial ownership of such entity.

    "You" (or "Your") shall mean an individual or Legal Entity
    exercising permissions granted by this License.

    "Source" form shall mean the preferred form for making modifications,
    including but not limited to software source code, documentation
    source, and configuration files.

    "Object" form shall mean any form resulting from mechanical
    transformation or translation of a Source form, including but
    not limited to compiled object code, generated documentation,
    and conversions to other media types.

    "Work" shall mean the work of authorship, whether in Source or
    Object form, made available under the License, as indicated by a
    copyright notice that is included in or attached to the work
    (an example is provided in the Appendix below).

    "Derivative Works" shall mean any work, whether in Source or Object
    form, that is based on (or derived from) the Work and for which the
    editorial revisions, annotations, elaborations, or other modifications
    represent, as a whole, an original work of authorship. For the purposes
    of this License, Derivative Works shall not include works that remain
    separable from, or merely link (or bind by name) to the interfaces of,
    the Work and Derivative Works thereof.

    "Contribution" shall mean any work of authorship, including
    the original version of the Work and any modifications or additions
    to that Work or Derivative Works thereof, that is intentionally
    submitted to Licensor for inclusion in the Work by the copyright owner
    or by an individual or Legal Entity authorized to submit on behalf of
    the copyright owner. For the purposes of this definition, "submitted"
    means any form of electronic, verbal, or written communication sent
    to the Licensor or its representatives, including but not limited to
    communication on electronic mailing lists, source code control systems,
    and issue tracking systems that are managed by, or on behalf of, the
    Licensor for the purpose of discussing and improving the Work, but
    excluding communication that is conspicuously marked or otherwise
    designated in writing by the copyright owner as "Not a Contribution."

    "Contributor" shall mean Licensor and any individual or Legal Entity
    on behalf of whom a Contribution has been received by Licensor and
    subsequently incorporated within the Work.

2.  Grant of Copyright License. Subject to the terms and conditions of
    this License, each Contributor hereby grants to You a perpetual,
    worldwide, non-exclusive, no-charge, royalty-free, irrevocable
    copyright license to reproduce, prepare Derivative Works of,
    publicly display, publicly perform, sublicense, and distribute the
    Work and such Derivative Works in Source or Object form.

3.  Grant of Patent License. Subject to the terms and conditions of
    this License, each Contributor hereby grants to You a perpetual,
    worldwide, non-exclusive, no-charge, royalty-free, irrevocable
    (except as stated in this section) patent license to make, have made,
    use, offer to sell, sell, import, and otherwise transfer the Work,
    where such license applies only to those patent claims licensable
    by such Contributor that are necessarily infringed by their
    Contribution(s) alone or by combination of their Contribution(s)
    with the Work to which such Contribution(s) was submitted. If You
    institute patent litigation against any entity (including a
    cross-claim or counterclaim in a lawsuit) alleging that the Work
    or a Contribution incorporated within the Work constitutes direct
    or contributory patent infringement, then any patent licenses
    granted to You under this License for that Work shall terminate
    as of the date such litigation is filed.

4.  Redistribution. You may reproduce and distribute copies of the
    Work or Derivative Works thereof in any medium, with or without
    modifications, and in Source or Object form, provided that You
    meet the following conditions:

    (a) You must give any other recipients of the Work or
    Derivative Works a copy of this License; and

    (b) You must cause any modified files to carry prominent notices
    stating that You changed the files; and

    (c) You must retain, in the Source form of any Derivative Works
    that You distribute, all copyright, patent, trademark, and
    attribution notices from the Source form of the Work,
    excluding those notices that do not pertain to any part of
    the Derivative Works; and

    (d) If the Work includes a "NOTICE" text file as part of its
    distribution, then any Derivative Works that You distribute must
    include a readable copy of the attribution notices contained
    within such NOTICE file, excluding those notices that do not
    pertain to any part of the Derivative Works, in at least one
    of the following places: within a NOTICE text file distributed
    as part of the Derivative Works; within the Source form or
    documentation, if provided along with the Derivative Works; or,
    within a display generated by the Derivative Works, if and
    wherever such third-party notices normally appear. The contents
    of the NOTICE file are for informational purposes only and
    do not modify the License. You may add Your own attribution
    notices within Derivative Works that You distribute, alongside
    or as an addendum to the NOTICE text from the Work, provided
    that such additional attribution notices cannot be construed
    as modifying the License.

    You may add Your own copyright statement to Your modifications and
    may provide additional or different license terms and conditions
    for use, reproduction, or distribution of Your modifications, or
    for any such Derivative Works as a whole, provided Your use,
    reproduction, and distribution of the Work otherwise complies with
    the conditions stated in this License.

5.  Submission of Contributions. Unless You explicitly state otherwise,
    any Contribution intentionally submitted for inclusion in the Work
    by You to the Licensor shall be under the terms and conditions of
    this License, without any additional terms or conditions.
    Notwithstanding the above, nothing herein shall supersede or modify
    the terms of any separate license agreement you may have executed
    with Licensor regarding such Contributions.

6.  Trademarks. This License does not grant permission to use the trade
    names, trademarks, service marks, or product names of the Licensor,
    except as required for reasonable and customary use in describing the
    origin of the Work and reproducing the content of the NOTICE file.

7.  Disclaimer of Warranty. Unless required by applicable law or
    agreed to in writing, Licensor provides the Work (and each
    Contributor provides its Contributions) on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
    implied, including, without limitation, any warranties or conditions
    of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
    PARTICULAR PURPOSE. You are solely responsible for determining the
    appropriateness of using or redistributing the Work and assume any
    risks associated with Your exercise of permissions under this License.

8.  Limitation of Liability. In no event and under no legal theory,
    whether in tort (including negligence), contract, or otherwise,
    unless required by applicable law (such as deliberate and grossly
    negligent acts) or agreed to in writing, shall any Contributor be
    liable to You for damages, including any direct, indirect, special,
    incidental, or consequential damages of any character arising as a
    result of this License or out of the use or inability to use the
    Work (including but not limited to damages for loss of goodwill,
    work stoppage, computer failure or malfunction, or any and all
    other commercial damages or losses), even if such Contributor
    has been advised of the possibility of such damages.

9.  Accepting Warranty or Additional Liability. While redistributing
    the Work or Derivative Works thereof, You may choose to offer,
    and charge a fee for, acceptance of support, warranty, indemnity,
    or other liability obligations and/or rights consistent with this
    License. However, in accepting such obligations, You may act only
    on Your own behalf and on Your sole responsibility, not on behalf
    of any other Contributor, and only if You agree to indemnify,
    defend, and hold each Contributor harmless for any liability
    incurred by, or claims asserted against, such Contributor by reason
    of your accepting any such warranty or additional liability.

END OF TERMS AND CONDITIONS

APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

Copyright 2024 Alex Garcia

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

### potion-retrieval-32M

The bundled weights are Hugging Face `minishlab/potion-retrieval-32M` at commit `6fc8051fab2a1e0ee76689cf08c853792ac285e7`. Files in `models/potion-retrieval-32M/`: `config.json`, `tokenizer.json`, `model.safetensors`. The ONNX file from that revision is not bundled.

That revision's file list has no LICENSE file. The model card `README.md` front matter says `license: mit`. The citation block in that card says `license = {MIT}`. The Hugging Face model record `cardData.license` is `mit`.

### potion-base-8M (test fixture, not in the built extension)

The repository also keeps Hugging Face `minishlab/potion-base-8M` at commit `bf8b056651a2c21b8d2565580b8569da283cab23` in `models/potion-base-8M/` for the provider-Model tests. It is not embedded in the default build.

That revision's file list has no LICENSE file. The model card `README.md` front matter says `license: mit`. The citation block in that card says `license = {MIT}`. The Hugging Face model record `cardData.license` is `mit`.

### Cargo crates

These are the crates in `Cargo.lock`. The license column is the `license` field from `cargo metadata` for that version. `model2vec-rs` has no `license` field; its `license_file` is `LICENSE`, and that file is the MIT license with the line `Copyright (c) 2025 The Minish Lab`.

Where a license expression offers a choice, RAGite takes MIT or Apache-2.0. Those texts are reproduced under sqlite-vec above. No resolved crate ships a NOTICE file.

| Crate | Version | License |
| --- | --- | --- |
| ahash | 0.8.12 | MIT OR Apache-2.0 |
| aho-corasick | 1.1.5 | Unlicense OR MIT |
| anyhow | 1.0.104 | MIT OR Apache-2.0 |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| base64 | 0.13.1 | MIT/Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| castaway | 0.2.4 | MIT |
| cc | 1.8.0 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| compact_str | 0.9.1 | MIT |
| console | 0.15.11 | MIT |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 |
| crunchy | 0.2.4 | MIT |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| darling | 0.20.11 | MIT |
| darling_core | 0.20.11 | MIT |
| darling_macro | 0.20.11 | MIT |
| dary_heap | 0.3.9 | MIT OR Apache-2.0 |
| derive_builder | 0.20.2 | MIT OR Apache-2.0 |
| derive_builder_core | 0.20.2 | MIT OR Apache-2.0 |
| derive_builder_macro | 0.20.2 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| either | 1.19.0 | MIT OR Apache-2.0 |
| encode_unicode | 1.0.0 | Apache-2.0 OR MIT |
| esaxx-rs | 0.1.10 | Apache-2.0 |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 |
| fnv | 1.0.7 | Apache-2.0 / MIT |
| futures-core | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| getrandom | 0.3.4 | MIT OR Apache-2.0 |
| half | 2.7.1 | MIT OR Apache-2.0 |
| ident_case | 1.0.1 | MIT/Apache-2.0 |
| indicatif | 0.17.11 | MIT |
| itertools | 0.14.0 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| js-sys | 0.3.106 | MIT OR Apache-2.0 |
| libc | 0.2.190 | MIT OR Apache-2.0 |
| libsqlite3-sys | 0.38.2 | MIT |
| log | 0.4.34 | MIT OR Apache-2.0 |
| macro_rules_attribute | 0.2.3 | Apache-2.0 OR MIT OR Zlib |
| macro_rules_attribute-proc_macro | 0.2.3 | Apache-2.0 OR MIT OR Zlib |
| matrixmultiply | 0.3.11 | MIT/Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 |
| model2vec-rs | 0.3.0 | MIT (LICENSE file; metadata license field is empty) |
| monostate | 0.1.18 | MIT OR Apache-2.0 |
| monostate-impl | 0.1.18 | MIT OR Apache-2.0 |
| ndarray | 0.15.6 | MIT OR Apache-2.0 |
| nom | 7.1.3 | MIT |
| num-complex | 0.4.6 | MIT OR Apache-2.0 |
| num-integer | 0.1.47 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| number_prefix | 0.4.0 | MIT |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| onig | 6.5.3 | MIT |
| onig_sys | 69.9.3 | MIT |
| paste | 1.0.15 | MIT OR Apache-2.0 |
| pastey | 0.2.3 | MIT OR Apache-2.0 |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT |
| ppv-lite86 | 0.2.21 | MIT OR Apache-2.0 |
| prettyplease | 0.2.37 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| rand | 0.9.5 | MIT OR Apache-2.0 |
| rand_chacha | 0.9.0 | MIT OR Apache-2.0 |
| rand_core | 0.9.5 | MIT OR Apache-2.0 |
| rawpointer | 0.2.1 | MIT/Apache-2.0 |
| rayon | 1.12.0 | MIT OR Apache-2.0 |
| rayon-cond | 0.4.0 | Apache-2.0/MIT |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 |
| regex | 1.13.1 | MIT OR Apache-2.0 |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 |
| safetensors | 0.5.3 | Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| slab | 0.4.12 | MIT |
| smallvec | 1.16.3 | MIT OR Apache-2.0 |
| spm_precompiled | 0.1.4 | Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| strsim | 0.11.1 | MIT |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |
| tokenizers | 0.21.4 | Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-normalization-alignments | 0.1.12 | MIT/Apache-2.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| unicode_categories | 0.1.1 | MIT OR Apache-2.0 |
| vcpkg | 0.2.15 | MIT/Apache-2.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 |
| web-time | 1.1.0 | MIT OR Apache-2.0 |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zmij | 1.0.23 | MIT |

`libsqlite3-sys` 0.38.2 is MIT. Its LICENSE file begins `Copyright (c) 2014 The rusqlite developers`. The extension enables the `loadable_extension` feature, so this crate supplies bindings and does not link a copy of SQLite.

`onig` 6.5.3 and `onig_sys` 69.9.3 are MIT. The `onig_sys` LICENSE.md copyright line is `Copyright (c) 2015 Will Speak <will@willspeak.me>, Ivan Ivashchenko <defuz@me.com>, and contributors.` `onig_sys` also compiles Oniguruma 6.9.10 from the crate directory `oniguruma/`. That code is not MIT. Its `COPYING` file says:

```
Oniguruma LICENSE
-----------------

Copyright (c) 2002-2021  K.Kosako  <kkosako0@gmail.com>
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:
1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in the
   documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
SUCH DAMAGE.
```

`unicode-ident` 1.0.27 is `(MIT OR Apache-2.0) AND Unicode-3.0`. The Unicode-3.0 notice in its `LICENSE-UNICODE` file is:

```
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2023 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.
```

### WHATWG HTML named character references

BSD 3-Clause. `crates/ragite-core/src/html_entities.rs` is the named character reference table from the WHATWG HTML Standard (https://html.spec.whatwg.org/entities.json, https://github.com/whatwg/html), arranged as a sorted lookup of names that end with `;`. The HTML Standard says portions incorporated into source code are licensed under BSD 3-Clause rather than CC-BY-4.0 (https://github.com/whatwg/html/blob/main/LICENSE). The table was rearranged; the names and code points are unchanged.

```
Copyright © WHATWG (Apple, Google, Mozilla, Microsoft).

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## Fuzz harness (not shipped)

The `fuzz/` workspace is not linked into the extension and is not part of `make test`. Its lockfile is `fuzz/Cargo.lock`. The crates below are in that lock and absent from the root `Cargo.lock`, except `getrandom` 0.4.3 and `r-efi` 6.0.0, which are extra versions of crates already listed above. The license column is the `license` field from `cargo metadata --manifest-path fuzz/Cargo.toml`. Where a license expression offers a choice, RAGite takes MIT or Apache-2.0. No resolved crate in that lock ships a NOTICE file.

| Crate | Version | License |
| --- | --- | --- |
| arbitrary | 1.5.0 | MIT OR Apache-2.0 |
| getrandom | 0.4.3 | MIT OR Apache-2.0 |
| jobserver | 0.1.35 | MIT OR Apache-2.0 |
| libfuzzer-sys | 0.4.13 | (MIT OR Apache-2.0) AND NCSA |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |

`libfuzzer-sys` 0.4.13 declares `license = "(MIT OR Apache-2.0) AND NCSA"` in its `Cargo.toml`. Its README says the files in `libfuzzer/` are NCSA and everything else is Apache-2.0 and MIT. The vendored `libfuzzer` sources carry `SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception` and point at <https://llvm.org/LICENSE.txt>. That file is the Apache License 2.0 (already reproduced under sqlite-vec) with the LLVM exceptions below, and it also prints the legacy University of Illinois/NCSA license. The harness links this engine only when someone runs the fuzz targets.

LLVM exceptions, from <https://llvm.org/LICENSE.txt>:

```
As an exception, if, as a result of your compiling your source code, portions
of this Software are embedded into an Object form of such source code, you
may redistribute such embedded portions in such Object form without complying
with the conditions of Sections 4(a), 4(b) and 4(d) of the License.

In addition, if you combine or link compiled forms of this Software with
software that is licensed under the GPLv2 ("Combined Software") and if a
court of competent jurisdiction determines that the patent provision (Section
3), the indemnity provision (Section 9) or other Section of the License
conflicts with the conditions of the GPLv2, you may retroactively and
prospectively choose to deem waived or otherwise exclude such Section(s) of
the License, but only in their entirety and only with respect to the Combined
Software.
```

University of Illinois/NCSA Open Source License, from the same file:

```
Copyright (c) 2003-2019 University of Illinois at Urbana-Champaign.
All rights reserved.

Developed by:

    LLVM Team

    University of Illinois at Urbana-Champaign

    http://llvm.org

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal with
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

    * Redistributions of source code must retain the above copyright notice,
      this list of conditions and the following disclaimers.

    * Redistributions in binary form must reproduce the above copyright notice,
      this list of conditions and the following disclaimers in the
      documentation and/or other materials provided with the distribution.

    * Neither the names of the LLVM Team, University of Illinois at
      Urbana-Champaign, nor the names of its contributors may be used to
      endorse or promote products derived from this Software without specific
      prior written permission.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
CONTRIBUTORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS WITH THE
SOFTWARE.
```

## External programs (run, not included)

`make test` runs the `sqlite3` command-line program and loads the RAGite extension into it. That program is not part of RAGite. The extension does not link a copy of SQLite.

`make test` also runs Python 3, Node.js, and Go. Python uses its standard-library `sqlite3` module. Node.js uses its built-in `node:sqlite` module. The Go test is cgo linked to the host `libsqlite3` and does not download modules. Those programs are not part of RAGite, and the tests do not install a RAGite package.

The manual Fuzz workflow installs `cargo-fuzz` 0.13.2 and runs it. That crate's `Cargo.toml` says `license = "MIT OR Apache-2.0"`. It is not part of the extension.

The CI `coverage` job runs `cargo clippy` from the Rust toolchain and, when the `SONAR_TOKEN` secret is set, the GitHub Action `SonarSource/sonarqube-scan-action` v8, which downloads and runs SonarScanner CLI 8.1.0.6389 to upload the analysis to SonarQube Cloud. Both the action and SonarScanner CLI are LGPL-3.0 according to their GitHub repositories. They are not part of RAGite and are not shipped.

The manual workflow `.github/workflows/mutants.yml` installs `cargo-mutants` 27.1.0 and runs it against `ragite-core`. That program is not part of RAGite and is not shipped. The published crate's `Cargo.toml` names the license MIT. Its `LICENSE` file states copyright 2021 Martin Pool.
