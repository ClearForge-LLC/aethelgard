# Route i handler: answers as the BME280 (chip-ID register 0xD0 reads 0x60).
# S3_FAULT: 0 = normal, 1 = withhold the bytes (the model pads zeros), 2 = corrupt (0x61).
S3_STATE = {"fault": 0, "reg": 0}

class Sensor:
    def __init__(self, d):
        self.d = d
    def write(self, data):
        if len(data) > 0:
            S3_STATE["reg"] = data[0]
    def read(self, n):
        f = S3_STATE["fault"]
        if S3_STATE["reg"] == 0xD0 and f != 1:
            self.d.EnqueueResponseByte(0x60 if f == 0 else 0x61)

def mc_s3_setup(path):
    d = monitor.Machine[path]
    s = Sensor(d)
    d.DataReceived += s.write
    d.ReadRequested += s.read

def mc_s3_fault(mode):
    S3_STATE["fault"] = int(mode)
