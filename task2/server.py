import socket
import threading
import os
import time
import math
from pathlib import Path
connections = []

stringPath = "uploads"
mutex = threading.Lock()
class NewClient(threading.Thread):
    def __init__(self, sock, address):
        threading.Thread.__init__(self)
        self.sock = sock
        self.address = address
        self.numOfPackets = 0
        self.currentPacket = 0
        self.numOfReceivedDataBytes = 0
        self.timeOfPackets = []

    def getTotalTime(self):
        # print(self.timeOfPackets)
        return self.timeOfPackets[-1] - self.timeOfPackets[0]

    def getCurrentSpeed(self):
        return math.ceil(self.numOfReceivedDataBytes / self.getTotalTime() * 100) / 100

    def getClientAddress(self):
        return self.address[0]

    def run(self):
        while True:
            try:
                data = self.sock.recv(1024)
                self.currentPacket += 1
                self.timeOfPackets.append(time.time())
                self.numOfReceivedDataBytes += len(data)
            except:
                print("Client disconnected " + self.address[0])
                success = 0
                self.sock.sendto(success.to_bytes(), self.address)
                mutex.acquire()
                connections.remove(self)
                mutex.release()
                break
            if self.currentPacket == 1:
                filePath = os.path.join(path, data.decode("utf-8"))
                file = open(filePath, "w+b")
            elif self.currentPacket == 2:
                 self.numOfPackets = data[0]
            elif self.numOfPackets == self.currentPacket:
                file.write(data)
                print("Client successfully sent a file and left the chanel " + self.address[0])
                success = 1
                self.sock.sendto(success.to_bytes(), self.address)
                self.sock.close()
                file.close()
                mutex.acquire()
                connections.remove(self)
                mutex.release()
                break
            else:
                file.write(data)

def newConnection(socke):
    while True:
        sock, addr = socke.accept()
        connections.append(NewClient(sock, addr))
        connections[-1].start()
        print("Client connected " + addr[0])

def displaySpeedOfConnection():
    if (len(connections) == 0):
        displayTimer = threading.Timer(1, displaySpeedOfConnection)
        displayTimer.start()
        return
    print("Client           Speed of Connection")
    mutex.acquire()
    for connection in connections:
        print(connection.getClientAddress() + "      " + str(connection.getCurrentSpeed()))
    mutex.release()
    displayTimer = threading.Timer(1, displaySpeedOfConnection)
    displayTimer.start()


path = Path("uploads")
displayTimer = threading.Timer(1, displaySpeedOfConnection)
def main():
    host = socket.gethostname()
    port = int(input("Server port: "))
    print(host + " is listening")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind((host, port))
    sock.listen(5)

    connectionsListener = threading.Thread(target = newConnection, args = [sock])
    connectionsListener.start()
    displayTimer.start()

    if not os.path.exists(path):
        os.mkdir(path)

main()