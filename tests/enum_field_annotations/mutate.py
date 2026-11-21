import struct

def u2(data, offset): return struct.unpack_from('>H', data, offset)[0]
def u4(data, offset): return struct.unpack_from('>I', data, offset)[0]
def fields(data):
    text = {}
    offset, index = 10, 1
    while index < u2(data, 8):
        tag = data[offset]
        offset += 1
        if tag == 1:
            size = u2(data, offset)
            text[index] = data[offset + 2:offset + 2 + size].decode('utf-8', errors='replace')
            offset += 2 + size
        elif tag in (3, 4, 9, 10, 11, 12, 17, 18): offset += 4
        elif tag in (5, 6): offset += 8; index += 1
        elif tag in (7, 8, 16, 19, 20): offset += 2
        elif tag == 15: offset += 3
        else: raise ValueError(tag)
        index += 1
    offset += 6
    offset += 2 + 2 * u2(data, offset)
    count = u2(data, offset)
    offset += 2
    result = []
    for _ in range(count):
        start = offset
        name = text[u2(data, offset + 2)]
        attributes = []
        count = u2(data, offset + 6)
        offset += 8
        for _ in range(count):
            attr_start = offset
            attr_name = text[u2(data, offset)]
            size = u4(data, offset + 2)
            offset += 6 + size
            attributes.append((attr_name, data[attr_start:offset]))
        result.append((name, start, offset, attributes))
    return text, result

def mutate(data, mode):
    text, records = fields(data)
    name, start, end, attributes = next(record for record in records if record[0] == 'FIRST')
    if mode == 'synthetic':
        return data[:start] + struct.pack('>H', u2(data, start) | 0x1000) + data[start + 2:]
    if mode == 'duplicate_annotation_table':
        payload = next(raw for name, raw in attributes if name == 'RuntimeVisibleAnnotations')
    elif mode == 'type_use':
        _, _, _, donor = next(record for record in records if record[0] == 'typed')
        payload = next(raw for name, raw in donor if name == 'RuntimeVisibleTypeAnnotations')
    elif mode == 'malformed_signature':
        names = {value: index for index, value in text.items()}
        payload = struct.pack('>HIH', names['Signature'], 2, names['FIRST'])
    else: raise ValueError(mode)
    return data[:start + 6] + struct.pack('>H', len(attributes) + 1) + data[start + 8:end] + payload + data[end:]
