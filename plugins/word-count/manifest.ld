; SPDX-License-Identifier: MPL-2.0
; manifest.ld — λδ package manifest for 'word-count'. Homoiconic: this is a λδ
; map literal, validated by nexia-core (lambdadelta::package::PackageManifest).
{
 :name "word-count"
 :version "0.1.0"
 :spdx "MPL-2.0"
 :tier :ayo                       ; :teranga core | :shield elevated-trust | :ayo community
 :description "Counts words per note and writes the :word-count attribute; the reference λδ package"
 :entry-point "src/main.ld"
 :capabilities [:notes/read :notes/write]
 ;; Configurator schema: the settings UI is generated from this map; no value
 ;; reaches the plugin unvalidated (PackageManifest::resolve_config).
 ;:config {:example {:type :int :default 0 :doc "an example setting"}}
 :tests ["test/main.test.ld"]
}
