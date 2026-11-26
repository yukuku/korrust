fun bad(x: String?): Int { try { x!! } catch (e: Exception) {}; return x.length }
