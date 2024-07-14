/*
 * Copyright (c) 2024 Yunshan Networks
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

#ifndef DF_JATTACH_H
#define DF_JATTACH_H

#include "config.h"

#define STRING_BUFFER_SIZE 2000
#define UNIX_PATH_MAX 108
#define JAVA_ADDR_STR_SIZE 13

/*
 * The address range of the 64-bit user space is from 0x0000000000000000
 * to 0x00007fffffffffff, which effectively uses only 48 bits. We use 13
 * bytes to represent the address string, with the last byte used as '\0'.
 */
typedef struct {
	char addr[JAVA_ADDR_STR_SIZE];
	bool is_verified;
} java_unload_addr_str_t;

typedef uint64_t(*agent_test_t) (void);

typedef struct options {
	char perf_map_path[PERF_PATH_SZ];
	char perf_log_path[PERF_PATH_SZ];
} options_t;

typedef struct receiver_args {
	pid_t pid;
	options_t *opts;
	int map_socket;
	int log_socket;
	FILE *map_fp;
	FILE *log_fp;
	int *attach_ret;
	bool *replay_done;
} receiver_args_t;

void clear_target_ns_tmp_file(const char *target_path);
int check_and_clear_target_ns(int pid, bool check_in_use);
void clear_target_ns_so(int pid, int target_ns_pid);
void clear_local_perf_files(int pid);
i64 get_target_symbol_file_sz(int pid, int ns_pid);
i64 get_local_symbol_file_sz(int pid, int ns_pid);
int target_symbol_file_access(int pid, int ns_pid);
#endif /* DF_JATTACH_H */
