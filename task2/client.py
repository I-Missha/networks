import math
import socket
import threading
import sys
import os
from time import sleep


def receive(sock):
    while (True):
        data = sock.recv(512)
        if data[0] == 1:
            print("Receiving ended successfully")
            sys.exit(0)


def main():
    host = input("Host: ")
    port = int(input("Port: "))
    fileName = input("File Name: ")

    # Attempt connection to server
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.connect((host, port))
    except:
        print("Could not make a connection to the server")
        input("Press enter to quit")
        sys.exit(0)

    receiveThread = threading.Thread(target = receive, args = [sock])
    receiveThread.start()

    sock.send(fileName.encode())
    sleep(0.1)
    file = open(fileName, "rb")
    fileSize = os.path.getsize(fileName)
    print((math.ceil(fileSize / 1024) + 2))
    sock.sendall((math.ceil(fileSize / 1024) + 2).to_bytes())
    sleep(0.1)
    for i in range(0, math.ceil(fileSize / 1024)):
        data = file.read(1024)
        sock.sendall(data)
        sleep(0.1)
if __name__ == "__main__":
    main()