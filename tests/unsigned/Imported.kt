package importedscalar
import externalvalue.Counter

fun arbitrary(first: Int, second: Int): Int = Counter(first).scramble(Counter(second)).raw
fun ordered(log: StringBuilder): Int = mark(log, "A", 2).scramble(mark(log, "B", 3)).raw
fun mark(log: StringBuilder, label: String, value: Int): Counter {
    log.append(label)
    return Counter(value)
}
fun arrayKind(): Class<*> = Counter(0).arrayKind()
