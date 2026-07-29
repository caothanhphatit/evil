#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

static void scan_region(int fd, uintptr_t start, uintptr_t end, const char *needle, int utf16) {
    unsigned char buf[65536 + 256];
    size_t length = strlen(needle);
    if (length == 0 || length > 255) return;
    for (uintptr_t address = start; address < end;) {
        size_t want = sizeof(buf) - 256;
        if (end - address < want) want = (size_t)(end - address);
        ssize_t got = pread(fd, buf, want, (off_t)address);
        if (got <= 0) return;
        for (ssize_t i = 0; i + (ssize_t)length <= got; i++) {
            int match = 1;
            for (size_t j = 0; j < length; j++) {
                size_t index = utf16 ? i + j * 2 : i + j;
                if (index >= (size_t)got || buf[index] != (unsigned char)needle[j]
                    || (utf16 && (index + 1 >= (size_t)got || buf[index + 1] != 0))) {
                    match = 0;
                    break;
                }
            }
            if (match) {
                printf("0x%lx\n", (unsigned long)(address + (uintptr_t)i));
                fflush(stdout);
            }
        }
        address += (uintptr_t)got;
    }
}

int main(int argc, char **argv) {
    if (argc != 3 && argc != 4) {
        fprintf(stderr, "usage: %s PID ASCII_NEEDLE [--utf16]\n", argv[0]);
        return 2;
    }
    char maps_path[64], mem_path[64];
    snprintf(maps_path, sizeof(maps_path), "/proc/%s/maps", argv[1]);
    snprintf(mem_path, sizeof(mem_path), "/proc/%s/mem", argv[1]);
    FILE *maps = fopen(maps_path, "r");
    int mem = open(mem_path, O_RDONLY | O_CLOEXEC);
    if (maps == NULL || mem < 0) {
        fprintf(stderr, "open failed: %s\n", strerror(errno));
        return 1;
    }
    char line[1024];
    while (fgets(line, sizeof(line), maps) != NULL) {
        uintptr_t start = 0, end = 0;
        char permissions[5] = {0};
        if (sscanf(line, "%lx-%lx %4s", &start, &end, permissions) != 3) continue;
        if (permissions[0] == 'r') scan_region(mem, start, end, argv[2], argc == 4 && strcmp(argv[3], "--utf16") == 0);
    }
    fclose(maps);
    close(mem);
    return 0;
}
