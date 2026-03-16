	.text
	.globl main
main:
	li	t5, 2
	xor	a7, t5, x0
	snez	a7, a7
	li	t5, 4
	xor	a6, t5, x0
	snez	a6, a6
	snez	t0, a7
	snez	t1, a6
	and	a5, t0, t1
	mv a0, a5
	ret
