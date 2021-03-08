use riscv_hypervisor_extension_proc_macro::generate_csr;
generate_csr!(
    "Hedeleg
1538
ex0,0,0,number,Instruction address misaligned
ex1,1,1,number,Instruction access fault
ex2,2,2,number,Illegal instruction 
ex3,3,3,number,Breakpoint 
ex4,4,4,number,Load address misaligned 
ex5,5,5,number,Load access fault 
ex6,6,6,number,Store/AMO address misaligned 
ex7,7,7,number,Store/AMO access fault 
ex8,8,8,number,Environment call from U-mode or VU-mode 
ex12,12,12,number,Instruction page fault 
ex13,13,13,number,Load page fault 
ex15,15,15,number,Store/AMO page fault 
end
Hypervisor Exception Delegation Register."
);
