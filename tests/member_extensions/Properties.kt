class Host(val suffix: String) {
        var stored: Int = 0
        val String.decorated: String get() = this + this@Host.suffix
        var String.amount: Int get() = length + stored; set(value) { stored = value }
        val Int.amount: Int get() = this + stored
        val String?.nullableSize: Int get() = this?.length ?: 0
        fun String.defaulted(value: String = this): String = value + suffix
        fun Int.mask(p0: Int = 1, p1: Int = 2, p2: Int = 3, p3: Int = 4, p4: Int = 5, p5: Int = 6, p6: Int = 7, p7: Int = 8, p8: Int = 9, p9: Int = 10, p10: Int = 11, p11: Int = 12, p12: Int = 13, p13: Int = 14, p14: Int = 15, p15: Int = 16, p16: Int = 17, p17: Int = 18, p18: Int = 19, p19: Int = 20, p20: Int = 21, p21: Int = 22, p22: Int = 23, p23: Int = 24, p24: Int = 25, p25: Int = 26, p26: Int = 27, p27: Int = 28, p28: Int = 29, p29: Int = 30, p30: Int = 31, p31: Int = 32): Int = this + p0 + p1 + p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9 + p10 + p11 + p12 + p13 + p14 + p15 + p16 + p17 + p18 + p19 + p20 + p21 + p22 + p23 + p24 + p25 + p26 + p27 + p28 + p29 + p30 + p31
        fun defaults(): Int = 1.mask()
        fun run(): String { "O".amount = 3; return "O".decorated + "x".amount + 2.amount + null.nullableSize }
    }
    object Holder { fun String.decorate(): String = this + "K"; fun run(): String = "O".decorate() }
    class Container { companion object { fun String.decorate(): String = this + "K"; val String.decorated: String get() = this + "K" }; fun run(): String = "O".decorate() }
    val String.topLength: Int get() = length
    fun Int.topMask(p0: Int = 1, p1: Int = 2, p2: Int = 3, p3: Int = 4, p4: Int = 5, p5: Int = 6, p6: Int = 7, p7: Int = 8, p8: Int = 9, p9: Int = 10, p10: Int = 11, p11: Int = 12, p12: Int = 13, p13: Int = 14, p14: Int = 15, p15: Int = 16, p16: Int = 17, p17: Int = 18, p18: Int = 19, p19: Int = 20, p20: Int = 21, p21: Int = 22, p22: Int = 23, p23: Int = 24, p24: Int = 25, p25: Int = 26, p26: Int = 27, p27: Int = 28, p28: Int = 29, p29: Int = 30, p30: Int = 31, p31: Int = 32): Int = this + p0 + p1 + p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9 + p10 + p11 + p12 + p13 + p14 + p15 + p16 + p17 + p18 + p19 + p20 + p21 + p22 + p23 + p24 + p25 + p26 + p27 + p28 + p29 + p30 + p31
    fun topDefaults(): Int = 1.topMask()
    