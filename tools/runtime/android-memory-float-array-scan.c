#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

enum {
    PAGE_BYTES = 4096,
    TARGET_LENGTH = 30,
    ARRAY_HEADER_BYTES = 32,
};

static int page_is_present(int pagemap_fd, uintptr_t address) {
    uint64_t entry = 0;
    off_t offset = (off_t)(address / PAGE_BYTES) * (off_t)sizeof(entry);
    ssize_t read_count = pread(pagemap_fd, &entry, sizeof(entry), offset);
    return read_count == (ssize_t)sizeof(entry) && (entry & (UINT64_C(1) << 63)) != 0;
}

static float minimum_value = 0.1f;
static float maximum_value = 3.0f;

static int plausible_values(const float *values) {
    int distinct = 0;
    int32_t seen[TARGET_LENGTH];
    for (int index = 0; index < TARGET_LENGTH; index += 1) {
        float value = values[index];
        if (!isfinite(value) || value < minimum_value || value > maximum_value) {
            return 0;
        }
        int32_t quantized = (int32_t)lrintf(value * 100000.0f);
        int already_seen = 0;
        for (int previous = 0; previous < distinct; previous += 1) {
            if (seen[previous] == quantized) {
                already_seen = 1;
                break;
            }
        }
        if (!already_seen) {
            seen[distinct] = quantized;
            distinct += 1;
        }
    }
    return distinct >= 8;
}

static void scan_page(uintptr_t address, const unsigned char *page) {
    const size_t required = ARRAY_HEADER_BYTES + TARGET_LENGTH * sizeof(float);
    for (size_t offset = 0; offset + required <= PAGE_BYTES; offset += sizeof(uintptr_t)) {
        uint64_t length = 0;
        memcpy(&length, page + offset + 24, sizeof(length));
        if (length != TARGET_LENGTH) {
            continue;
        }
        float values[TARGET_LENGTH];
        memcpy(values, page + offset + ARRAY_HEADER_BYTES, sizeof(values));
        if (!plausible_values(values)) {
            continue;
        }
        printf("0x%" PRIxPTR, address + offset);
        for (int index = 0; index < TARGET_LENGTH; index += 1) {
            printf(" %0.9g", values[index]);
        }
        putchar('\n');
        fflush(stdout);
    }
}

int main(int argc, char **argv) {
    if (argc != 2 && argc != 4) {
        fprintf(stderr, "usage: %s PID [MIN_FLOAT MAX_FLOAT]\n", argv[0]);
        return 2;
    }
    if (argc == 4) {
        minimum_value = strtof(argv[2], NULL);
        maximum_value = strtof(argv[3], NULL);
        if (!isfinite(minimum_value) || !isfinite(maximum_value) || minimum_value > maximum_value) {
            fprintf(stderr, "invalid float range\n");
            return 2;
        }
    }

    char maps_path[64];
    char mem_path[64];
    char pagemap_path[64];
    snprintf(maps_path, sizeof(maps_path), "/proc/%s/maps", argv[1]);
    snprintf(mem_path, sizeof(mem_path), "/proc/%s/mem", argv[1]);
    snprintf(pagemap_path, sizeof(pagemap_path), "/proc/%s/pagemap", argv[1]);

    FILE *maps = fopen(maps_path, "r");
    int mem_fd = open(mem_path, O_RDONLY | O_CLOEXEC);
    int pagemap_fd = open(pagemap_path, O_RDONLY | O_CLOEXEC);
    if (maps == NULL || mem_fd < 0 || pagemap_fd < 0) {
        fprintf(stderr, "failed to open target memory files: %s\n", strerror(errno));
        return 1;
    }

    char line[1024];
    unsigned char page[PAGE_BYTES];
    uint64_t scanned_pages = 0;
    while (fgets(line, sizeof(line), maps) != NULL) {
        uintptr_t start = 0;
        uintptr_t end = 0;
        char permissions[5] = {0};
        if (sscanf(line, "%" SCNxPTR "-%" SCNxPTR " %4s", &start, &end, permissions) != 3) {
            continue;
        }
        if (permissions[0] != 'r' || permissions[1] != 'w') {
            continue;
        }
        for (uintptr_t address = start; address < end; address += PAGE_BYTES) {
            if (!page_is_present(pagemap_fd, address)) {
                continue;
            }
            ssize_t read_count = pread(mem_fd, page, sizeof(page), (off_t)address);
            if (read_count != (ssize_t)sizeof(page)) {
                continue;
            }
            scan_page(address, page);
            scanned_pages += 1;
        }
    }

    fprintf(stderr, "scanned resident bytes: %" PRIu64 "\n", scanned_pages * PAGE_BYTES);
    fclose(maps);
    close(mem_fd);
    close(pagemap_fd);
    return 0;
}
