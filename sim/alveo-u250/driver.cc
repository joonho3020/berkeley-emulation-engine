#include <iostream>
#include <fcntl.h>   // For open()
#include <unistd.h>  // For pwrite() and close()
#include <stdio.h>
#include <cstring>   // For strerror()

int main() {
    off_t offset = 0x2000; // Offset at which to write
    char data[65] = "CAFFCCFFDFCDBFCFCBFFCCFFDFCDBFCFCBFFCCFFDFCDBFCFCBFFCCFFDFCDBFCF";

    printf("strnlen(data): %d\n", strlen(data));

    const char *filePath = "/dev/xdma0_h2c_0";
    int fd = open(filePath, O_WRONLY, 0644);
    if (fd == -1) {
      std::cerr << "Error opening file: " << strerror(errno) << std::endl;
      return 1;
    }
    printf("write fd: %d\n", fd);

    ssize_t bytesWritten = pwrite(fd, data, strlen(data), offset);
    if (bytesWritten == -1) {
      std::cerr << "Error writing to file: " << strerror(errno) << std::endl;
      close(fd);
      return 1;
    }

    std::cout << data << std::endl;
    std::cout << "Wrote " << bytesWritten << " bytes to " << filePath << " at offset " << offset << "." << std::endl;

    const char *read_filePath = "/dev/xdma0_c2h_0";
    int rd_fd = open(read_filePath, O_RDONLY, 0644);
    if (rd_fd == -1) {
        std::cerr << "Error opening file: " << strerror(errno) << std::endl;
        return 1;
    }

    char read_data[64];

    ssize_t bytesRead = pread(rd_fd, read_data, strlen(data), offset);
    if (bytesRead == -1) {
        std::cerr << "Error writing to file: " << strerror(errno) << std::endl;
        close(rd_fd);
        return 1;
    }

    std::cout << "Read " << bytesRead << " bytes from " << read_filePath << " at offset " << offset << "." << std::endl;
    printf("read_data %s\n", read_data);
/* std::cout << "Readdata: " << read_data << std::endl; */
    printf("read_data ptr: %p\n", read_data);

    for (int i = 0; i < 128; i++) {
      printf("%c", read_data[i]);
    }
    printf("\n");

// close(fd);
/* close(rd_fd); */
    return 0;
}
