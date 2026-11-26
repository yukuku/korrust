"""Verify relocated inline singletons against resolved classpath and output ownership."""
import hashlib
from pathlib import Path
import struct
import zipfile

ATTRIBUTE = 'KorrustInlineOrigin'


def fnv(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return value


def mutf_decode(data):
    units = []
    position = 0
    while position < len(data):
        byte = data[position]
        position += 1
        if 0 < byte < 128:
            units.append(byte)
        elif byte & 0xe0 == 0xc0:
            other = data[position]
            position += 1
            if other & 0xc0 != 0x80:
                raise ValueError('Invalid modified UTF-8')
            units.append(((byte & 31) << 6) | (other & 63))
        elif byte & 0xf0 == 0xe0:
            second, third = data[position:position + 2]
            position += 2
            if second & 0xc0 != 0x80 or third & 0xc0 != 0x80:
                raise ValueError('Invalid modified UTF-8')
            units.append(((byte & 15) << 12) | ((second & 63) << 6) | (third & 63))
        else:
            raise ValueError('Invalid modified UTF-8')
    return b''.join(struct.pack('>H', unit) for unit in units).decode('utf-16-be', errors='surrogatepass')


def mutf_encode(value):
    units = value.encode('utf-16-be', errors='surrogatepass')
    data = bytearray()
    for position in range(0, len(units), 2):
        unit = int.from_bytes(units[position:position + 2], 'big')
        if 0 < unit < 128:
            data.append(unit)
        elif unit < 2048:
            data.extend((0xc0 | unit >> 6, 0x80 | unit & 63))
        else:
            data.extend((0xe0 | unit >> 12, 0x80 | unit >> 6 & 63, 0x80 | unit & 63))
    return b'\x01' + struct.pack('>H', len(data)) + data


class Reader:
    def __init__(self, data, position=0):
        self.data, self.position = data, position

    def take(self, size):
        start = self.position
        self.position += size
        if size < 0 or self.position > len(self.data):
            raise ValueError('Truncated classfile or origin record')
        return self.data[start:self.position]

    def number(self, size):
        return int.from_bytes(self.take(size), 'big')


def parse(data):
    reader = Reader(data)
    if reader.take(4) != b'\xca\xfe\xba\xbe':
        raise ValueError('Invalid classfile magic')
    reader.take(4)
    count = reader.number(2)
    pool, raw = [None] * count, [b''] * count
    index = 1
    while index < count:
        start = reader.position
        tag = reader.number(1)
        if tag == 1:
            value = mutf_decode(reader.take(reader.number(2)))
        elif tag in (7, 8, 16, 19, 20):
            value = reader.number(2)
        elif tag in (9, 10, 11, 12, 17, 18):
            value = (reader.number(2), reader.number(2))
        elif tag in (3, 4):
            value = reader.take(4)
        elif tag in (5, 6):
            value = reader.take(8)
        elif tag == 15:
            value = reader.take(3)
        else:
            raise ValueError('Unknown constant-pool tag')
        pool[index] = (tag, value)
        raw[index] = data[start:reader.position]
        index += 2 if tag in (5, 6) else 1
    pool_end = reader.position
    flags, owner, parent = reader.number(2), reader.number(2), reader.number(2)
    interfaces = [reader.number(2) for _ in range(reader.number(2))]

    def text(index):
        tag, value = pool[index]
        if tag != 1:
            raise ValueError('Expected UTF-8 constant')
        return value

    def class_name(index):
        tag, value = pool[index]
        if tag != 7:
            raise ValueError('Expected class constant')
        return text(value)

    def attributes():
        entries = []
        for _ in range(reader.number(2)):
            start = reader.position
            name = reader.number(2)
            payload_start = reader.position + 4
            payload = reader.take(reader.number(4))
            entries.append(dict(name=text(name), index=name, payload=payload,
                                start=start, payload_start=payload_start, end=reader.position))
        return entries

    groups = []
    for _ in range(2):
        members = []
        for _ in range(reader.number(2)):
            access, name, descriptor = reader.number(2), text(reader.number(2)), text(reader.number(2))
            members.append(dict(access=access, name=name, descriptor=descriptor, attributes=attributes()))
        groups.append(members)
    attributes_start = reader.position
    attrs = attributes()
    if reader.position != len(data) or len({entry['name'] for entry in attrs}) != len(attrs):
        raise ValueError('Trailing classfile bytes or duplicate class attributes')
    return dict(pool=pool, raw=raw, pool_end=pool_end, name=class_name(owner), parent=class_name(parent) if parent else None,
                flags=flags, interfaces=[class_name(index) for index in interfaces], fields=groups[0], methods=groups[1],
                attributes=attrs, attributes_start=attributes_start, text=text, class_name=class_name)


def instructions(code):
    position = 0
    while position < len(code):
        start, opcode = position, code[position]
        if opcode in (0xaa, 0xab):
            position += 1
            position += (-position) & 3
            reader = Reader(code, position)
            reader.take(4)
            if opcode == 0xaa:
                low = int.from_bytes(reader.take(4), 'big', signed=True)
                high = int.from_bytes(reader.take(4), 'big', signed=True)
                count = high - low + 1
                if count < 0:
                    raise ValueError('Invalid tableswitch')
                reader.take(count * 4)
            else:
                count = int.from_bytes(reader.take(4), 'big', signed=True)
                reader.take(count * 8)
            position = reader.position
        elif opcode == 0xc4:
            if position + 1 >= len(code) or code[position + 1] not in (*range(0x15, 0x1a), *range(0x36, 0x3b), 0x84, 0xa9):
                raise ValueError('Invalid wide instruction')
            position += 6 if code[position + 1] == 0x84 else 4
        elif opcode in (0x10, 0x12, *range(0x15, 0x1a), *range(0x36, 0x3b), 0xa9, 0xbc):
            position += 2
        elif opcode in (0x11, 0x13, 0x14, 0x84, *range(0x99, 0xa9), *range(0xb2, 0xb9), 0xbb, 0xbd, 0xc0, 0xc1, 0xc6, 0xc7):
            position += 3
        elif opcode in (0xb9, 0xba, 0xc8, 0xc9):
            position += 5
        elif opcode == 0xc5:
            position += 4
        elif opcode <= 0xc9:
            position += 1
        else:
            raise ValueError('Invalid JVM opcode')
        if position > len(code):
            raise ValueError('Truncated JVM instruction')
        yield start, opcode


def source_file(info):
    values = [entry for entry in info['attributes'] if entry['name'] == 'SourceFile']
    if len(values) != 1 or len(values[0]['payload']) != 2:
        raise ValueError('Missing or invalid original SourceFile')
    return info['text'](int.from_bytes(values[0]['payload'], 'big'))


def resolve_class(classpath, owner):
    resource = owner + '.class'
    for path in map(Path, classpath):
        if path.is_dir():
            entry = path / resource
            if entry.is_file():
                return entry.read_bytes(), str(entry)
        elif path.is_file():
            with zipfile.ZipFile(path) as archive:
                if resource in archive.namelist():
                    return archive.read(resource), str(path) + '!/' + resource
    raise ValueError('Inline origin absent from resolved classpath: ' + owner)


def reconstruct(original, output_info, payload):
    origin = parse(original)
    marker = Reader(payload)
    if marker.number(2) != 1:
        raise ValueError('Unsupported inline origin version')
    owner, caller = [marker.take(marker.number(2)).decode('utf-8') for _ in range(2)]
    offset, digest = marker.number(8), marker.number(8)
    if marker.position != len(payload) or digest != fnv(original) or owner != origin['name']:
        raise ValueError('Inline origin record does not match resolved original bytes')
    renamed = output_info['name']
    prefix = caller + '$inlineDefault$' + str(offset) + '$'
    suffix = renamed.removeprefix(prefix)
    if not renamed.startswith(prefix) or len(suffix) != 16 or any(char not in '0123456789abcdef' for char in suffix):
        raise ValueError('Inline origin owner or offset does not match helper name')
    fields, methods = origin['fields'], origin['methods']
    if (origin['flags'] & 0x11 != 0x11 or origin['parent'] not in ('java/lang/Object', 'kotlin/jvm/internal/Lambda')
            or len(fields) != 1 or fields[0]['name'] != 'INSTANCE' or fields[0]['descriptor'] != 'L' + owner + ';'
            or fields[0]['access'] & 0x19 != 0x19 or len(origin['interfaces']) != 1
            or not origin['interfaces'][0].startswith('kotlin/jvm/functions/Function')
            or not any(method['name'] == '<init>' and method['descriptor'] == '()V' and method['access'] & 1 for method in methods)
            or any(method['name'] not in ('invoke', '<init>', '<clinit>') for method in methods)):
        raise ValueError('Origin is not a supported capture-free inline default singleton')
    pool, raw = origin['pool'], origin['raw'][:]
    literals = {value for entry in pool if entry and entry[0] == 8 for value in [entry[1]]}
    for index, entry in enumerate(pool):
        if entry and entry[0] == 1:
            name = entry[1]
            new = renamed if name == owner else name.replace('L' + owner + ';', 'L' + renamed + ';')
            if new != name:
                if index in literals:
                    raise ValueError('Relocation changes a string literal')
                raw[index] = mutf_encode(new)
    body = bytearray(original[origin['pool_end']:origin['attributes_start']])
    for method in methods:
        codes = [entry for entry in method['attributes'] if entry['name'] == 'Code']
        if len(codes) != 1:
            raise ValueError('Inline singleton method lacks executable code')
        entry = codes[0]
        code_reader = Reader(entry['payload'], 4)
        code = code_reader.take(code_reader.number(4))
        for position, opcode in instructions(code):
            if opcode != 0xb8:
                continue
            tag, reference = pool[int.from_bytes(code[position + 1:position + 3], 'big')]
            if tag != 10:
                continue
            class_index, signature_index = reference
            _, (name_index, descriptor_index) = pool[signature_index]
            name = origin['text'](name_index)
            if origin['class_name'](class_index) != 'kotlin/jvm/internal/Intrinsics':
                continue
            if name == 'reifiedOperationMarker' or name == 'needClassReification' and method['name'] != '<clinit>':
                raise ValueError('Origin requires unsupported reified regeneration')
            if name == 'needClassReification':
                if origin['text'](descriptor_index) != '()V':
                    raise ValueError('Invalid reification marker signature')
                start = entry['payload_start'] + 8 + position - origin['pool_end']
                body[start:start + 3] = b'\0\0\0'
    attrs = [entry for entry in origin['attributes'] if entry['name'] not in ('InnerClasses', 'EnclosingMethod', 'NestHost', 'NestMembers')]
    if any(entry['name'] == ATTRIBUTE for entry in attrs):
        raise ValueError('Nested inline origin records are unsupported')
    result = original[:8] + struct.pack('>H', len(pool) + 1) + b''.join(raw) + mutf_encode(ATTRIBUTE) + body
    result += struct.pack('>H', len(attrs) + 1)
    result += b''.join(original[entry['start']:entry['end']] for entry in attrs)
    result += struct.pack('>HI', len(pool), len(payload)) + payload
    return result, dict(origin=owner, caller=caller, offset=offset, source_file=source_file(origin), original_sha256=hashlib.sha256(original).hexdigest())


def verify_inline_origin(data, classpath):
    info = parse(data)
    entries = [entry for entry in info['attributes'] if entry['name'] == ATTRIBUTE]
    if not entries:
        return None
    if len(entries) != 1:
        raise ValueError('Duplicate inline origin attributes')
    payload = entries[0]['payload']
    marker = Reader(payload)
    marker.take(2)
    owner = marker.take(marker.number(2)).decode('utf-8')
    original, resource = resolve_class(classpath, owner)
    expected, detail = reconstruct(original, info, payload)
    if expected != data:
        raise ValueError('Inline helper differs from exact resolved-library relocation')
    if source_file(info) != detail['source_file']:
        raise ValueError('Inline relocation changes SourceFile')
    return dict(detail, resource=resource, helper=info['name'], helper_sha256=hashlib.sha256(data).hexdigest())


def verify_owned_class_origins(output, source, classpath, records):
    """Require source provenance or exact inline relocation for every owned class.

    `records` must first pass the output ownership index validation, including
    the source identity, checksum, content hashes, and complete output set.
    """
    classes = {}
    origins = {}
    for name in records:
        if not name.endswith('.class'):
            continue
        data = (Path(output) / name).read_bytes()
        info = parse(data)
        if name != info['name'] + '.class':
            raise ValueError('Owned class path does not match declared class name')
        classes[info['name']] = info
        origin = verify_inline_origin(data, classpath)
        if origin:
            origins[name] = origin
        elif source_file(info) != Path(source).name:
            raise ValueError('Owned class lacks source or verified inline provenance: ' + name)
    for name, origin in origins.items():
        caller = classes.get(origin['caller'])
        if caller is None or source_file(caller) != Path(source).name:
            raise ValueError('Inline origin caller is not owned by the compiled source')
        references = {caller['class_name'](index) for index, entry in enumerate(caller['pool']) if entry and entry[0] == 7}
        if origin['helper'] not in references:
            raise ValueError('Owned inline caller does not reference its relocated helper')
    return origins
