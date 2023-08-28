package config

import (
	"io/ioutil"
	"reflect"
	"testing"

	yaml "gopkg.in/yaml.v2"
)

type baseConfig struct {
	Config ingesterConfig `yaml:"ingester"`
}

type ingesterConfig struct {
	ExportersCfg ExportersCfg `yaml:"exporters"`
}

func TestConfig(t *testing.T) {
	ingesterCfg := baseConfig{}
	configBytes, _ := ioutil.ReadFile("./config_test.yaml")
	err := yaml.Unmarshal(configBytes, &ingesterCfg)
	if err != nil {
		t.Fatalf("yaml unmarshal failed: %v", err)
	}
	expect := baseConfig{
		Config: ingesterConfig{
			ExportersCfg: ExportersCfg{
				Enabled:                     false,
				ExportDatas:                 []string{"cbpf-net-span", "ebpf-sys-span"},
				ExportDataTypes:             []string{"service_info", "tracing_info", "network_layer", "flow_info", "transport_layer", "application_layer", "metrics"},
				ExportCustomK8sLabelsRegexp: "",
				ExportOnlyWithTraceID:       false,
				OtlpExporterCfg: OtlpExporterConfig{
					Enabled:          true,
					Addr:             "127.0.0.1:4317",
					QueueCount:       4,
					QueueSize:        100000,
					ExportBatchCount: 32,
					GrpcHeaders: map[string]string{
						"key1": "value1",
						"key2": "value2",
					},
				},
			},
		},
	}
	if !reflect.DeepEqual(expect, ingesterCfg) {
		t.Fatalf("yaml unmarshal not equal, expect: %v, got: %v", expect, ingesterCfg)
	}
}
