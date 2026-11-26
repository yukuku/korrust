fun bad(x: String?): Int { val later: () -> Int = { x!!.length }; return x.length }
