package qualifiedconsumer

import qv.p.Root

fun javaWork(): Int {
    qv.p.State.count = 4
    qv.p.State.count += 3
    val before = qv.p.State.count++
    return before * 10 + qv.p.State.count + qv.p.State.add(2)
}

fun kotlinWork(): Int {
    qv.kotlin.Singleton.count = 5
    qv.kotlin.Singleton.count += 2
    qv.kotlin.Host.count = 11
    qv.kotlin.Host.count++
    qv.kotlin.Host.Companion.count += 3
    return qv.kotlin.Singleton.add(1) * 100 + qv.kotlin.Host.add(2)
}

fun parameterShadow(qv: Root): Int {
    qv.p.State.count = 30
    qv.p.State.count += 2
    return qv.p.State.count++
}

fun captureShadow(qv: Root): Int {
    val update: () -> Int = { qv.p.State.count += 3; qv.p.State.count }
    return update()
}

class MemberShadow(val qv: Root) {
    fun work(): Int { qv.p.State.count = 41; return qv.p.State.count }
}

open class ShadowBase(val qv: Root)
class InheritedShadow(qv: Root) : ShadowBase(qv) {
    fun work(): Int { qv.p.State.count = 42; return qv.p.State.count }
}

fun nestedWork(): Int = Container.Options(71).value
