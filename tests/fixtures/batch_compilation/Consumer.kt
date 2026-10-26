package batch.consumer

import batch.sample.inferred
import batch.sample.inferredProperty
import batch.sample.combined
import batch.sample.weighted
import batch.sample.identity
import batch.sample.ownPrivate

fun main() {
    check(inferred() == 12)
    check(inferredProperty == 9)
    check(combined() == 30)
    check(weighted(value = 4) == 12)
    check(identity("metadata") == "metadata")
    check(ownPrivate() == 13)
    check(batch.sample.Holder.DEFAULT.value + batch.sample.Holder.DEFAULT.bonus == 9)
    check(batch.left.Same(4).value + batch.right.Same(9).value == 13)
    println("consumer-ok:30")
}
