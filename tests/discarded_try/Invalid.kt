fun invalid(flag: Boolean): Int = try { if (flag) Effects.left() else Effects.right() } finally { Effects.right() }
