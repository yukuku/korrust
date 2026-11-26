fun consumeDiscard(): Int { discardWhen(1); discardBound(1); val action = lambdaWhen(1); action(); return branchReturn(0) }
