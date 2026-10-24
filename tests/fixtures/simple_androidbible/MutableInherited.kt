package simple.mutable

import simple.mutable.MutableSurface

interface MutableInherited : MutableSurface {
    override var count: Int

    fun readCount(): Int = count
    fun readLabel(): String = label
}
