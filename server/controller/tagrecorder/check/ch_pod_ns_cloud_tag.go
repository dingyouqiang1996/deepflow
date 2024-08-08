/*
 * Copyright (c) 2023 Yunshan Networks
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

package tagrecorder

import (
	"github.com/deepflowio/deepflow/server/controller/db/mysql"
	"github.com/deepflowio/deepflow/server/controller/tagrecorder"
)

type ChPodNSCloudTag struct {
	UpdaterBase[mysql.ChPodNSCloudTag, CloudTagKey]
}

func NewChPodNSCloudTag() *ChPodNSCloudTag {
	updater := &ChPodNSCloudTag{
		UpdaterBase[mysql.ChPodNSCloudTag, CloudTagKey]{
			resourceTypeName: RESOURCE_TYPE_CH_POD_NS_CLOUD_TAG,
		},
	}
	updater.dataGenerator = updater
	return updater
}

func (p *ChPodNSCloudTag) generateNewData() (map[CloudTagKey]mysql.ChPodNSCloudTag, bool) {
	var podNamespaces []mysql.PodNamespace
	err := p.db.Unscoped().Find(&podNamespaces).Error
	if err != nil {
		log.Errorf(dbQueryResourceFailed(p.resourceTypeName, err), p.db.LogPrefixORGID)
		return nil, false
	}

	keyToItem := make(map[CloudTagKey]mysql.ChPodNSCloudTag)
	for _, podNamespace := range podNamespaces {
		teamID, err := tagrecorder.GetTeamID(podNamespace.Domain, podNamespace.SubDomain)
		if err != nil {
			log.Errorf("resource(%s) %s, resource: %#v", p.resourceTypeName, err.Error(), podNamespace, p.db.LogPrefixORGID)
		}

		for k, v := range podNamespace.CloudTags {
			key := CloudTagKey{
				ID:  podNamespace.ID,
				Key: k,
			}
			keyToItem[key] = mysql.ChPodNSCloudTag{
				ID:          podNamespace.ID,
				Key:         k,
				Value:       v,
				TeamID:      teamID,
				DomainID:    tagrecorder.DomainToDomainID[podNamespace.Domain],
				SubDomainID: tagrecorder.SubDomainToSubDomainID[podNamespace.SubDomain],
			}
		}
	}
	return keyToItem, true
}

func (p *ChPodNSCloudTag) generateKey(dbItem mysql.ChPodNSCloudTag) CloudTagKey {
	return CloudTagKey{ID: dbItem.ID, Key: dbItem.Key}
}

func (p *ChPodNSCloudTag) generateUpdateInfo(oldItem, newItem mysql.ChPodNSCloudTag) (map[string]interface{}, bool) {
	updateInfo := make(map[string]interface{})
	if oldItem.Value != newItem.Value {
		updateInfo["value"] = newItem.Value
	}
	if len(updateInfo) > 0 {
		return updateInfo, true
	}
	return nil, false
}
