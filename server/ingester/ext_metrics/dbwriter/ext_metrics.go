/*
 * Copyright (c) 2022 Yunshan Networks
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package dbwriter

import (
	"github.com/deepflowys/deepflow/server/libs/ckdb"
	"github.com/deepflowys/deepflow/server/libs/pool"
	"github.com/deepflowys/deepflow/server/libs/zerodoc"
)

type ExtMetrics struct {
	Timestamp uint32 // s

	Tag zerodoc.Tag

	Database  string
	TableName string

	TagNames  []string
	TagValues []string

	MetricsFloatNames  []string
	MetricsFloatValues []float64
}

func (m *ExtMetrics) WriteBlock(block *ckdb.Block) error {
	if err := m.Tag.WriteBlock(block, m.Timestamp); err != nil {
		return err
	}

	if err := block.WriteArrayString(m.TagNames); err != nil {
		return err
	}
	if err := block.WriteArrayString(m.TagValues); err != nil {
		return err
	}
	if err := block.WriteArrayString(m.MetricsFloatNames); err != nil {
		return err
	}
	if err := block.WriteArrayFloat64(m.MetricsFloatValues); err != nil {
		return err
	}

	return nil
}

func (m *ExtMetrics) Columns() []*ckdb.Column {
	columns := zerodoc.GenTagColumns(m.Tag.Code)
	columns = append(columns,
		ckdb.NewColumn("tag_names", ckdb.ArrayString).SetComment("额外的tag"),
		ckdb.NewColumn("tag_values", ckdb.ArrayString).SetComment("额外的tag对应的值"),
		ckdb.NewColumn("metrics_float_names", ckdb.ArrayString).SetComment("额外的float类型metrics"),
		ckdb.NewColumn("metrics_float_values", ckdb.ArrayFloat64).SetComment("额外的float metrics值"),
	)
	return columns
}

func (m *ExtMetrics) Release() {
	ReleaseExtMetrics(m)
}

func (m *ExtMetrics) GenCKTable(ttl int) *ckdb.Table {
	timeKey := "time"
	cluster := ckdb.DF_CLUSTER
	engine := ckdb.MergeTree

	orderKeys := []string{}
	if m.Tag.Code&zerodoc.L3EpcID != 0 {
		orderKeys = append(orderKeys, "l3_epc_id")
	}
	if m.Tag.Code&zerodoc.IP != 0 {
		orderKeys = append(orderKeys, "ip4")
		orderKeys = append(orderKeys, "ip6")
	}
	orderKeys = append(orderKeys, timeKey)

	return &ckdb.Table{
		Database:        m.Database,
		LocalName:       m.TableName + ckdb.LOCAL_SUBFFIX,
		GlobalName:      m.TableName,
		Columns:         m.Columns(),
		TimeKey:         timeKey,
		TTL:             ttl,
		PartitionFunc:   ckdb.TimeFuncTwelveHour,
		Engine:          engine,
		Cluster:         cluster,
		OrderKeys:       orderKeys,
		PrimaryKeyCount: len(orderKeys),
	}
}

var extMetricsPool = pool.NewLockFreePool(func() interface{} {
	return &ExtMetrics{
		Tag: zerodoc.Tag{
			Field: &zerodoc.Field{},
		},
		TagNames:           make([]string, 0, 4),
		TagValues:          make([]string, 0, 4),
		MetricsFloatNames:  make([]string, 0, 4),
		MetricsFloatValues: make([]float64, 0, 4),
	}
})

func AcquireExtMetrics() *ExtMetrics {
	return extMetricsPool.Get().(*ExtMetrics)
}

func ReleaseExtMetrics(m *ExtMetrics) {
	*m.Tag.Field = zerodoc.Field{}
	m.Tag.Code = 0
	m.TagNames = m.TagNames[:0]
	m.TagValues = m.TagValues[:0]
	m.MetricsFloatNames = m.MetricsFloatNames[:0]
	m.MetricsFloatValues = m.MetricsFloatValues[:0]
	extMetricsPool.Put(m)
}
