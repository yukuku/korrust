package qualifiedalias

import qv.p.State.ROOT as qv

fun work(): Int {
    qv.p.State.count = 50
    qv.p.State.count += 4
    return qv.p.State.count
}
