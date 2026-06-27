function fact(n) { let t=0; for(let x=1;x<=n;x++) { let r=1; for(let i=1;i<=x;i++) r*=i; t+=r } return t }
console.log(fact(12))
