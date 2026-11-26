package qualifiedpropertyalias

import qv.kotlin.root as qv

fun work(): Int {
    qv.p.State.count = 60
    qv.p.State.count += 4
    return qv.p.State.count
}
