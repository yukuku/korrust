package outside

import simple.mutable.ReadOnlyBase

fun invalid(base: ReadOnlyBase) {
    base.value = 1
}
