fun bad(x: String?): Int { var y = x; y!!; y = null; return y.length }
