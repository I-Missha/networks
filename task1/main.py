import threading
import socket
import struct
import time
import uuid
import platform
from threading import Timer

class SyncMap():
    def __init__(self):
        self.hash_table = {}
        self.id = {}
        self.mutex = threading.Lock()

    def pop(self, key):
        self.mutex.acquire()
        self.hash_table.pop(key, None)
        self.mutex.release()

    def read(self):
        self.mutex.acquire()
        temp = self.hash_table.copy()
        self.mutex.release()
        return temp

    def write(self, key, value):
        self.mutex.acquire()
        self.hash_table[key] = value
        self.hash_table[key].start()
        self.mutex.release()


    def reset(self, address, value):
        self.hash_table[address].cancel()
        self.hash_table[address] = value
        self.hash_table[address].start()

    def isContained(self, address):
        return address in self.hash_table


class ListeningThread(threading.Thread):
    def __init__(self, mymap, sock, grp, port):
        threading.Thread.__init__(self)
        self.MCAST_GRP = grp
        self.MCAST_PORT = port
        self.map = mymap
        self.sock = sock

    def timeIsOut(self, arg):
        self.map.pop(arg)
        print(arg + "left the group")

    def run(self):

        while True:
            data, address = self.sock.recvfrom(1024)
            data = uuid.UUID(bytes=data)
            if not (self.map.isContained(data)):
                self.map.write(data, Timer(5, self.timeIsOut, data))
                continue

            self.map.reset(data, Timer(5, self.timeIsOut, data))

class PullingThread(threading.Thread):
    def __init__(self, sock, grp, port):
        threading.Thread.__init__(self)
        self.sock = sock
        self.MCAST_GRP = grp
        self.MCAST_PORT = port
        self.id = uuid.uuid4()

    def run(self):
        while True:
            time.sleep(3)
            self.sock.sendto(self.id.bytes, (self.MCAST_GRP, self.MCAST_PORT))


def main():
    # MCAST_GRP = '224.23.23.23'
    MCAST_GRP = 'ff15:7079:7468:6f6e:6465:6d6f:6d63:6173'
    MCAST_PORT = 5317
    MULTICAST_TTL = 2
    # Look up multicast group address in name server and find out IP version
    addrinfo = socket.getaddrinfo(MCAST_GRP, None)[0]

    # Create a socket
    s = socket.socket(addrinfo[0], socket.SOCK_DGRAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

    group_bin = socket.inet_pton(addrinfo[0], addrinfo[4][0])
    # Join group
    if addrinfo[0] == socket.AF_INET:  # IPv4
        mreq = group_bin + struct.pack('=I', socket.INADDR_ANY)
        s.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
    else:
        mreq = group_bin + struct.pack('@I', 0)
        s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_JOIN_GROUP, mreq)

    ttl_bin = struct.pack('@i', MULTICAST_TTL)
    if addrinfo[0] == socket.AF_INET:  # IPv4
        s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, ttl_bin)
    else:
        s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_MULTICAST_HOPS, ttl_bin)

    if addrinfo[0] == socket.AF_INET:  # IPv4
        s.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_LOOP, 1)

    else:
        s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_MULTICAST_LOOP, 1)

    if (platform.system() == "Windows"):
        s.bind(('0.0.0.0', MCAST_PORT))
    else:
        s.bind((MCAST_GRP, MCAST_PORT))

    map = SyncMap()
    listen_thread = ListeningThread(map, s, MCAST_GRP, MCAST_PORT)
    listen_thread.start()
    pulling_thread = PullingThread(s, MCAST_GRP, MCAST_PORT)
    pulling_thread.start()
    while True:
        time.sleep(3)
        print("-----new-iter--")
        for iter in map.read():
            print(iter)
        print("-end-new-iter--\n")

if __name__ == '__main__':
    main()
