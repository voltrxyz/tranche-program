/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/voltr_tranching.json`.
 */
export type VoltrTranching = {
  "address": "HcqA5TydrpKnVotpjp5fNFdKp8sXktcp92Pz6Z6hCjpF",
  "metadata": {
    "name": "voltrTranching",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "acceptAuthority",
      "discriminator": [
        107,
        86,
        198,
        91,
        33,
        12,
        107,
        160
      ],
      "accounts": [
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "pendingAuthority",
          "signer": true
        }
      ],
      "args": []
    },
    {
      "name": "cancelWithdraw",
      "discriminator": [
        112,
        53,
        226,
        58,
        158,
        30,
        37,
        168
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true,
          "relations": [
            "withdrawReceipt"
          ]
        },
        {
          "name": "trancheConfig",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          },
          "relations": [
            "withdrawReceipt"
          ]
        },
        {
          "name": "trancheAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "withdrawReceipt",
          "docs": [
            "The withdraw receipt to cancel. Validated: belongs to this config and this user."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  119,
                  105,
                  116,
                  104,
                  100,
                  114,
                  97,
                  119,
                  95,
                  114,
                  101,
                  99,
                  101,
                  105,
                  112,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              },
              {
                "kind": "account",
                "path": "withdraw_receipt.receipt_id",
                "account": "withdrawReceipt"
              }
            ]
          }
        },
        {
          "name": "trancheMint",
          "docs": [
            "Tranche token mint (senior or junior, must match the receipt's tranche)."
          ]
        },
        {
          "name": "trancheTokenEscrow",
          "docs": [
            "Tranche token escrow owned by tranche_authority (source of returned tokens)."
          ],
          "writable": true
        },
        {
          "name": "userTrancheAta",
          "docs": [
            "User's tranche token account (destination for returned tokens)."
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "completeWithdraw",
      "discriminator": [
        172,
        129,
        141,
        17,
        95,
        253,
        251,
        98
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true,
          "relations": [
            "withdrawReceipt"
          ]
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          },
          "relations": [
            "withdrawReceipt"
          ]
        },
        {
          "name": "trancheAuthority",
          "docs": [
            "validated by seeds constraint."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (mutable for burn if senior withdrawal)."
          ],
          "writable": true
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (mutable for burn if junior withdrawal;",
            "also needed for inline crank auto-pause check)."
          ],
          "writable": true
        },
        {
          "name": "trancheTokenEscrow",
          "docs": [
            "Tranche token escrow holding the escrowed tokens (senior or junior,",
            "depending on the receipt). Owned by tranche_authority."
          ],
          "writable": true
        },
        {
          "name": "assetEscrow",
          "docs": [
            "Asset escrow owned by tranche_authority (receives assets from vault CPI,",
            "then transfers to user)."
          ],
          "writable": true
        },
        {
          "name": "userAssetAta",
          "docs": [
            "User's asset token account (receives final withdrawn assets)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (LP tokens burned via CPI)."
          ],
          "writable": true
        },
        {
          "name": "assetMint",
          "docs": [
            "Underlying asset mint."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ],
          "writable": true
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank; mutable for CPI burn)."
          ],
          "writable": true
        },
        {
          "name": "protocol"
        },
        {
          "name": "vaultAssetIdleAta",
          "writable": true
        },
        {
          "name": "vaultAssetIdleAuth",
          "writable": true
        },
        {
          "name": "withdrawReceipt",
          "docs": [
            "The withdraw receipt to complete. Closed atomically (rent -> user)."
          ],
          "writable": true
        },
        {
          "name": "voltrVaultProgram",
          "address": "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "crankAccounting",
      "discriminator": [
        64,
        223,
        74,
        30,
        9,
        166,
        183,
        26
      ],
      "accounts": [
        {
          "name": "payer",
          "docs": [
            "Permissionless -- any fee payer can trigger accounting."
          ],
          "signer": true
        },
        {
          "name": "trancheConfig",
          "docs": [
            "The TrancheConfig to update. PDA seeds bind it to the vault."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault (read-only, no CPI)."
          ]
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint -- used to read total supply for pricing."
          ]
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "Our LP token custody account -- used to read our LP balance."
          ]
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint -- supply used for auto-pause price check."
          ]
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint -- supply used for auto-pause price check."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "depositJunior",
      "discriminator": [
        4,
        71,
        156,
        106,
        25,
        232,
        177,
        157
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "docs": [
            "validated by seeds constraint."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (read-only, needed for account validation)."
          ]
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (mutable for mint_to)."
          ],
          "writable": true
        },
        {
          "name": "userAssetAta",
          "docs": [
            "User's asset token account (source of deposit)."
          ],
          "writable": true
        },
        {
          "name": "userJuniorAta",
          "docs": [
            "User's junior tranche token account (receives minted tokens)."
          ],
          "writable": true
        },
        {
          "name": "assetEscrow",
          "docs": [
            "Asset escrow owned by tranche_authority (intermediate for CPI deposit)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (receives LP from vault)."
          ],
          "writable": true
        },
        {
          "name": "assetMint",
          "docs": [
            "Underlying asset mint."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ],
          "writable": true
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank; mutable for CPI)."
          ],
          "writable": true
        },
        {
          "name": "protocol"
        },
        {
          "name": "vaultAssetIdleAta",
          "writable": true
        },
        {
          "name": "vaultAssetIdleAuth"
        },
        {
          "name": "vaultLpMintAuth"
        },
        {
          "name": "voltrVaultProgram",
          "address": "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "depositSenior",
      "discriminator": [
        218,
        97,
        124,
        117,
        212,
        161,
        218,
        219
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "docs": [
            "validated by seeds constraint."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (mutable for mint_to)."
          ],
          "writable": true
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (read-only, needed for inline crank auto-pause check)."
          ]
        },
        {
          "name": "userAssetAta",
          "docs": [
            "User's asset token account (source of deposit)."
          ],
          "writable": true
        },
        {
          "name": "userSeniorAta",
          "docs": [
            "User's senior tranche token account (receives minted tokens)."
          ],
          "writable": true
        },
        {
          "name": "assetEscrow",
          "docs": [
            "Asset escrow owned by tranche_authority (intermediate for CPI deposit)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (receives LP from vault)."
          ],
          "writable": true
        },
        {
          "name": "assetMint",
          "docs": [
            "Underlying asset mint."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ],
          "writable": true
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank; mutable for CPI)."
          ],
          "writable": true
        },
        {
          "name": "protocol"
        },
        {
          "name": "vaultAssetIdleAta",
          "writable": true
        },
        {
          "name": "vaultAssetIdleAuth"
        },
        {
          "name": "vaultLpMintAuth"
        },
        {
          "name": "voltrVaultProgram",
          "address": "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializeTrancheConfig",
      "discriminator": [
        58,
        73,
        224,
        50,
        220,
        233,
        56,
        147
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault"
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  101,
                  110,
                  105,
                  111,
                  114,
                  95,
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "juniorMint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  106,
                  117,
                  110,
                  105,
                  111,
                  114,
                  95,
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "assetMint"
        },
        {
          "name": "vaultLpMint"
        },
        {
          "name": "vaultLpCustody",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "trancheAuthority"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ]
              },
              {
                "kind": "account",
                "path": "vaultLpMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "assetEscrow",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "trancheAuthority"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ]
              },
              {
                "kind": "account",
                "path": "assetMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "aprFeed",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  112,
                  114,
                  95,
                  102,
                  101,
                  101,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "initializeTrancheConfigParams"
            }
          }
        }
      ]
    },
    {
      "name": "instantWithdrawJunior",
      "discriminator": [
        26,
        2,
        53,
        45,
        92,
        198,
        156,
        31
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "docs": [
            "validated by seeds constraint."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (read-only, needed for inline crank auto-pause check)."
          ]
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (mutable for burn)."
          ],
          "writable": true
        },
        {
          "name": "userJuniorAta",
          "docs": [
            "User's junior tranche token account (source of tokens to burn)."
          ],
          "writable": true
        },
        {
          "name": "userAssetAta",
          "docs": [
            "User's asset token account (receives withdrawn assets)."
          ],
          "writable": true
        },
        {
          "name": "assetEscrow",
          "docs": [
            "Asset escrow owned by tranche_authority (receives assets from vault CPI,",
            "then transfers to user)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (LP tokens burned via CPI)."
          ],
          "writable": true
        },
        {
          "name": "assetMint",
          "docs": [
            "Underlying asset mint."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ],
          "writable": true
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank; mutable for CPI burn)."
          ],
          "writable": true
        },
        {
          "name": "protocol"
        },
        {
          "name": "vaultAssetIdleAta",
          "writable": true
        },
        {
          "name": "vaultAssetIdleAuth",
          "writable": true
        },
        {
          "name": "voltrVaultProgram",
          "address": "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "isTokenAmount",
          "type": "bool"
        }
      ]
    },
    {
      "name": "instantWithdrawSenior",
      "discriminator": [
        168,
        133,
        149,
        247,
        16,
        206,
        212,
        93
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "docs": [
            "validated by seeds constraint."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (mutable for burn)."
          ],
          "writable": true
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (read-only, needed for inline crank auto-pause check)."
          ]
        },
        {
          "name": "userSeniorAta",
          "docs": [
            "User's senior tranche token account (source of tokens to burn)."
          ],
          "writable": true
        },
        {
          "name": "userAssetAta",
          "docs": [
            "User's asset token account (receives withdrawn assets)."
          ],
          "writable": true
        },
        {
          "name": "assetEscrow",
          "docs": [
            "Asset escrow owned by tranche_authority (receives assets from vault CPI,",
            "then transfers to user)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (LP tokens burned via CPI)."
          ],
          "writable": true
        },
        {
          "name": "assetMint",
          "docs": [
            "Underlying asset mint."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ],
          "writable": true
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank; mutable for CPI burn)."
          ],
          "writable": true
        },
        {
          "name": "protocol"
        },
        {
          "name": "vaultAssetIdleAta",
          "writable": true
        },
        {
          "name": "vaultAssetIdleAuth",
          "writable": true
        },
        {
          "name": "voltrVaultProgram",
          "address": "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "isTokenAmount",
          "type": "bool"
        }
      ]
    },
    {
      "name": "pushAprObservation",
      "discriminator": [
        102,
        61,
        43,
        73,
        244,
        77,
        154,
        112
      ],
      "accounts": [
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "aprFeed",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  112,
                  114,
                  95,
                  102,
                  101,
                  101,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "vault"
        },
        {
          "name": "vaultLpMint"
        }
      ],
      "args": []
    },
    {
      "name": "requestWithdrawJunior",
      "discriminator": [
        253,
        39,
        216,
        54,
        119,
        138,
        171,
        42
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (read-only, needed for inline crank auto-pause check)."
          ]
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (read-only, needed for supply)."
          ]
        },
        {
          "name": "userJuniorAta",
          "docs": [
            "User's junior tranche token account (source of tokens to escrow)."
          ],
          "writable": true
        },
        {
          "name": "juniorTokenEscrow",
          "docs": [
            "Junior tranche token escrow owned by tranche_authority (receives escrowed tokens)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (needed for inline crank NAV calc)."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ]
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank)."
          ]
        },
        {
          "name": "withdrawReceipt",
          "docs": [
            "The withdraw receipt PDA to be created."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  119,
                  105,
                  116,
                  104,
                  100,
                  114,
                  97,
                  119,
                  95,
                  114,
                  101,
                  99,
                  101,
                  105,
                  112,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "isTokenAmount",
          "type": "bool"
        }
      ]
    },
    {
      "name": "requestWithdrawSenior",
      "discriminator": [
        223,
        124,
        240,
        197,
        61,
        0,
        209,
        64
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "seniorMint",
          "docs": [
            "Senior tranche token mint (read-only, needed for supply)."
          ]
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (read-only, needed for inline crank auto-pause check)."
          ]
        },
        {
          "name": "userSeniorAta",
          "docs": [
            "User's senior tranche token account (source of tokens to escrow)."
          ],
          "writable": true
        },
        {
          "name": "seniorTokenEscrow",
          "docs": [
            "Senior tranche token escrow owned by tranche_authority (receives escrowed tokens)."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (needed for inline crank NAV calc)."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ]
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank)."
          ]
        },
        {
          "name": "withdrawReceipt",
          "docs": [
            "The withdraw receipt PDA to be created."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  119,
                  105,
                  116,
                  104,
                  100,
                  114,
                  97,
                  119,
                  95,
                  114,
                  101,
                  99,
                  101,
                  105,
                  112,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "isTokenAmount",
          "type": "bool"
        }
      ]
    },
    {
      "name": "setPaused",
      "discriminator": [
        91,
        60,
        125,
        192,
        176,
        225,
        166,
        218
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true,
          "relations": [
            "trancheConfig"
          ]
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "paused",
          "type": "u8"
        }
      ]
    },
    {
      "name": "transferAuthority",
      "discriminator": [
        48,
        169,
        76,
        72,
        229,
        180,
        55,
        161
      ],
      "accounts": [
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "authority",
          "signer": true,
          "relations": [
            "trancheConfig"
          ]
        },
        {
          "name": "newAuthority",
          "docs": [
            "The new authority that must call `accept_authority` to complete the transfer."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "updateTrancheConfig",
      "discriminator": [
        17,
        66,
        9,
        47,
        165,
        108,
        26,
        255
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true,
          "relations": [
            "trancheConfig"
          ]
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "field",
          "type": {
            "defined": {
              "name": "trancheConfigField"
            }
          }
        }
      ]
    },
    {
      "name": "withdrawReserve",
      "discriminator": [
        165,
        158,
        228,
        5,
        114,
        119,
        194,
        14
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true,
          "relations": [
            "trancheConfig"
          ]
        },
        {
          "name": "trancheConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "trancheAuthority",
          "docs": [
            "validated by seeds constraint."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  97,
                  110,
                  99,
                  104,
                  101,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "trancheConfig"
              }
            ]
          }
        },
        {
          "name": "assetEscrow",
          "docs": [
            "Asset escrow owned by tranche_authority (receives assets from vault CPI,",
            "then transfers to authority destination)."
          ],
          "writable": true
        },
        {
          "name": "destinationAta",
          "docs": [
            "Authority's destination ATA for receiving withdrawn reserve assets."
          ],
          "writable": true
        },
        {
          "name": "vaultLpCustody",
          "docs": [
            "LP token custody owned by tranche_authority (LP tokens burned via CPI)."
          ],
          "writable": true
        },
        {
          "name": "assetMint",
          "docs": [
            "Underlying asset mint."
          ]
        },
        {
          "name": "juniorMint",
          "docs": [
            "Junior tranche token mint (needed for inline crank auto-pause check)."
          ]
        },
        {
          "name": "vault",
          "docs": [
            "The underlying Voltr Vault."
          ],
          "writable": true
        },
        {
          "name": "vaultLpMint",
          "docs": [
            "Vault LP token mint (supply needed for inline crank; mutable for CPI burn)."
          ],
          "writable": true
        },
        {
          "name": "protocol"
        },
        {
          "name": "vaultAssetIdleAta",
          "writable": true
        },
        {
          "name": "vaultAssetIdleAuth",
          "writable": true
        },
        {
          "name": "voltrVaultProgram",
          "address": "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "aprFeed",
      "discriminator": [
        215,
        67,
        17,
        249,
        6,
        65,
        20,
        160
      ]
    },
    {
      "name": "trancheConfig",
      "discriminator": [
        132,
        228,
        43,
        251,
        224,
        23,
        48,
        65
      ]
    },
    {
      "name": "vault",
      "discriminator": [
        211,
        8,
        232,
        43,
        2,
        152,
        117,
        119
      ]
    },
    {
      "name": "withdrawReceipt",
      "discriminator": [
        132,
        238,
        44,
        182,
        207,
        9,
        124,
        140
      ]
    }
  ],
  "events": [
    {
      "name": "cancelWithdrawEvent",
      "discriminator": [
        154,
        149,
        104,
        96,
        154,
        168,
        180,
        243
      ]
    },
    {
      "name": "completeWithdrawEvent",
      "discriminator": [
        203,
        101,
        137,
        176,
        17,
        211,
        84,
        72
      ]
    },
    {
      "name": "crankAccountingEvent",
      "discriminator": [
        216,
        76,
        161,
        184,
        132,
        181,
        143,
        16
      ]
    },
    {
      "name": "depositJuniorEvent",
      "discriminator": [
        79,
        170,
        49,
        216,
        243,
        203,
        180,
        130
      ]
    },
    {
      "name": "depositSeniorEvent",
      "discriminator": [
        194,
        187,
        170,
        48,
        187,
        90,
        233,
        207
      ]
    },
    {
      "name": "instantWithdrawEvent",
      "discriminator": [
        207,
        159,
        219,
        138,
        183,
        30,
        108,
        106
      ]
    },
    {
      "name": "requestWithdrawEvent",
      "discriminator": [
        8,
        19,
        138,
        15,
        73,
        48,
        140,
        155
      ]
    },
    {
      "name": "updateTrancheConfigEvent",
      "discriminator": [
        130,
        50,
        116,
        25,
        155,
        107,
        204,
        76
      ]
    },
    {
      "name": "withdrawReserveEvent",
      "discriminator": [
        124,
        48,
        121,
        249,
        27,
        160,
        41,
        35
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "seniorDepositsPaused",
      "msg": "Senior deposits are paused"
    },
    {
      "code": 6001,
      "name": "juniorDepositsPaused",
      "msg": "Junior deposits are paused"
    },
    {
      "code": 6002,
      "name": "seniorWithdrawalsPaused",
      "msg": "Senior withdrawals are paused"
    },
    {
      "code": 6003,
      "name": "juniorWithdrawalsPaused",
      "msg": "Junior withdrawals are paused"
    },
    {
      "code": 6004,
      "name": "insufficientCoverage",
      "msg": "Insufficient coverage ratio for senior deposit"
    },
    {
      "code": 6005,
      "name": "juniorRatioBreach",
      "msg": "Junior withdrawal would breach minimum junior/senior ratio"
    },
    {
      "code": 6006,
      "name": "cooldownNotElapsed",
      "msg": "Withdrawal cooldown period has not elapsed"
    },
    {
      "code": 6007,
      "name": "zeroAmount",
      "msg": "Amount must be greater than zero"
    },
    {
      "code": 6008,
      "name": "mathOverflow",
      "msg": "Math overflow in checked arithmetic"
    },
    {
      "code": 6009,
      "name": "unauthorized",
      "msg": "Caller is not the authority"
    },
    {
      "code": 6010,
      "name": "invalidFee",
      "msg": "Fee parameter exceeds maximum allowed value"
    },
    {
      "code": 6011,
      "name": "invalidRiskConfig",
      "msg": "Risk exponent must be a whole-number value that fits in u32"
    },
    {
      "code": 6012,
      "name": "juniorShortfall",
      "msg": "Junior tranche shortfall detected, auto-pause triggered"
    },
    {
      "code": 6013,
      "name": "vaultWaitingPeriodNotZero",
      "msg": "Vault has non-zero withdrawal waiting period (unsupported in v1)"
    },
    {
      "code": 6014,
      "name": "invalidVault",
      "msg": "Vault account mismatch or invalid owner"
    },
    {
      "code": 6015,
      "name": "invalidPauseFlags",
      "msg": "Invalid pause flags: only bits 0-3 are valid"
    },
    {
      "code": 6016,
      "name": "cooldownMustBeZero",
      "msg": "Instant withdrawal requires cooldown_seconds == 0"
    }
  ],
  "types": [
    {
      "name": "aprFeed",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "head",
            "docs": [
              "Ring buffer head pointer (next write index)."
            ],
            "type": "u16"
          },
          {
            "name": "count",
            "docs": [
              "Number of observations stored (max `MAX_APR_OBSERVATIONS`)."
            ],
            "type": "u16"
          },
          {
            "name": "padding",
            "docs": [
              "Padding to align observations to 8-byte boundary."
            ],
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "observations",
            "docs": [
              "Circular buffer of APR observations."
            ],
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "aprObservation"
                  }
                },
                72
              ]
            }
          }
        ]
      }
    },
    {
      "name": "aprObservation",
      "docs": [
        "A single APR observation snapshot."
      ],
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vaultNav",
            "docs": [
              "Vault NAV at observation time (Decimal bits)."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "vaultLpSupply",
            "docs": [
              "Vault LP token supply at observation time (Decimal bits)."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "timestamp",
            "docs": [
              "Unix timestamp of the observation."
            ],
            "type": "i64"
          },
          {
            "name": "padding",
            "docs": [
              "Reserved padding for future expansion and stable 48-byte slots."
            ],
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "cancelWithdrawEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "receiptId",
            "type": "u64"
          },
          {
            "name": "tokensReturned",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "completeWithdrawEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "receiptId",
            "type": "u64"
          },
          {
            "name": "isSenior",
            "type": "bool"
          },
          {
            "name": "finalValue",
            "type": "u64"
          },
          {
            "name": "tokensBurned",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "crankAccountingEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "delta",
            "type": "i128"
          },
          {
            "name": "seniorNav",
            "type": "u128"
          },
          {
            "name": "juniorNav",
            "type": "u128"
          },
          {
            "name": "reserveNav",
            "type": "u128"
          },
          {
            "name": "totalNav",
            "type": "u128"
          },
          {
            "name": "baseApr",
            "type": "u16"
          },
          {
            "name": "coverageRatio",
            "type": "u128"
          },
          {
            "name": "slot",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "depositJuniorEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "assetAmount",
            "type": "u64"
          },
          {
            "name": "tokensMinted",
            "type": "u64"
          },
          {
            "name": "juniorNav",
            "type": "u128"
          },
          {
            "name": "coverageRatio",
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "depositSeniorEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "assetAmount",
            "type": "u64"
          },
          {
            "name": "tokensMinted",
            "type": "u64"
          },
          {
            "name": "seniorNav",
            "type": "u128"
          },
          {
            "name": "coverageRatio",
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "feeConfiguration",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "managerPerformanceFee",
            "docs": [
              "Manager performance fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "adminPerformanceFee",
            "docs": [
              "Admin performance fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "managerManagementFee",
            "docs": [
              "Manager management fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "adminManagementFee",
            "docs": [
              "Admin management fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "redemptionFee",
            "docs": [
              "The redemption fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "issuanceFee",
            "docs": [
              "The issuance fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "protocolPerformanceFee",
            "docs": [
              "Protocol performance fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "protocolManagementFee",
            "docs": [
              "Protocol management fee in basis points (BPS)."
            ],
            "type": "u16"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feeState",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accumulatedLpManagerFees",
            "docs": [
              "The accumulated manager fees in the vault."
            ],
            "type": "u64"
          },
          {
            "name": "accumulatedLpAdminFees",
            "docs": [
              "The accumulated admin fees in the vault."
            ],
            "type": "u64"
          },
          {
            "name": "accumulatedLpProtocolFees",
            "docs": [
              "The accumulated protocol fees in the vault."
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feeUpdate",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lastPerformanceFeeUpdateTs",
            "docs": [
              "The timestamp when the performance fees were last updated."
            ],
            "type": "u64"
          },
          {
            "name": "lastManagementFeeUpdateTs",
            "docs": [
              "The timestamp when the management fees were last updated."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "highWaterMark",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "highestAssetPerLpDecimalBits",
            "docs": [
              "The highest recorded total asset value per share"
            ],
            "type": "u128"
          },
          {
            "name": "lastUpdatedTs",
            "docs": [
              "The timestamp when the high water mark was last updated"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use"
            ],
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "initializeTrancheConfigParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "aprFloorBps",
            "type": "u16"
          },
          {
            "name": "riskXBps",
            "type": "u16"
          },
          {
            "name": "riskYBps",
            "type": "u16"
          },
          {
            "name": "riskK",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "reserveFeeBps",
            "type": "u16"
          },
          {
            "name": "seniorExitFeeBps",
            "type": "u16"
          },
          {
            "name": "juniorExitFeeBps",
            "type": "u16"
          },
          {
            "name": "minJrSrRatioBps",
            "type": "u16"
          },
          {
            "name": "minCoverageForMintBps",
            "type": "u16"
          },
          {
            "name": "minCoverageBufferBps",
            "type": "u16"
          },
          {
            "name": "seniorCooldownSeconds",
            "type": "u64"
          },
          {
            "name": "juniorCooldownSeconds",
            "type": "u64"
          },
          {
            "name": "maxObservationAgeSeconds",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "instantWithdrawEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "isSenior",
            "type": "bool"
          },
          {
            "name": "assetAmount",
            "type": "u64"
          },
          {
            "name": "tokensBurned",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "lockedProfitState",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lastUpdatedLockedProfit",
            "type": "u64"
          },
          {
            "name": "lastReport",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "requestWithdrawEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "receiptId",
            "type": "u64"
          },
          {
            "name": "isSenior",
            "type": "bool"
          },
          {
            "name": "tokensEscrowed",
            "type": "u64"
          },
          {
            "name": "assetValue",
            "type": "u128"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "trancheConfig",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault",
            "docs": [
              "Underlying Voltr Vault address."
            ],
            "type": "pubkey"
          },
          {
            "name": "authority",
            "docs": [
              "Admin authority."
            ],
            "type": "pubkey"
          },
          {
            "name": "pendingAuthority",
            "docs": [
              "Pending authority for 2-step transfer."
            ],
            "type": "pubkey"
          },
          {
            "name": "seniorMint",
            "docs": [
              "Senior tranche token mint."
            ],
            "type": "pubkey"
          },
          {
            "name": "juniorMint",
            "docs": [
              "Junior tranche token mint."
            ],
            "type": "pubkey"
          },
          {
            "name": "lpCustody",
            "docs": [
              "ATA holding vault LP tokens (custody)."
            ],
            "type": "pubkey"
          },
          {
            "name": "assetMint",
            "docs": [
              "Underlying asset mint (e.g., USDC)."
            ],
            "type": "pubkey"
          },
          {
            "name": "totalNav",
            "docs": [
              "Total net asset value."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "seniorNav",
            "docs": [
              "Senior tranche NAV."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "juniorNav",
            "docs": [
              "Junior tranche NAV."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "reserveNav",
            "docs": [
              "Protocol reserve NAV (accumulated performance fees)."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "seniorTargetIndex",
            "docs": [
              "Compound growth index (Decimal bits, init: `Decimal::ONE`)."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "indexTimestamp",
            "docs": [
              "Last index update timestamp."
            ],
            "type": "i64"
          },
          {
            "name": "aprFloorBps",
            "docs": [
              "Minimum senior APR in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "riskXBps",
            "docs": [
              "Risk premium constant X in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "riskYBps",
            "docs": [
              "Risk premium constant Y in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "padding0",
            "docs": [
              "Explicit padding for stable layout."
            ],
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "riskK",
            "docs": [
              "Risk premium exponent K (Decimal bits)."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "baseAprBps",
            "docs": [
              "Current vault base APR in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "reserveFeeBps",
            "docs": [
              "Performance fee in basis points (max 2000 = 20%)."
            ],
            "type": "u16"
          },
          {
            "name": "seniorExitFeeBps",
            "docs": [
              "Senior withdrawal fee in basis points (max 100 = 1%)."
            ],
            "type": "u16"
          },
          {
            "name": "juniorExitFeeBps",
            "docs": [
              "Junior withdrawal fee in basis points (max 100 = 1%)."
            ],
            "type": "u16"
          },
          {
            "name": "minJrSrRatioBps",
            "docs": [
              "Minimum junior/senior ratio in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "minCoverageForMintBps",
            "docs": [
              "Minimum coverage ratio to allow senior deposits in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "minCoverageBufferBps",
            "docs": [
              "Coverage buffer for senior minting in basis points."
            ],
            "type": "u16"
          },
          {
            "name": "padding1",
            "docs": [
              "Explicit padding for stable layout."
            ],
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "seniorCooldownSeconds",
            "docs": [
              "Senior withdrawal cooldown in seconds."
            ],
            "type": "u64"
          },
          {
            "name": "juniorCooldownSeconds",
            "docs": [
              "Junior withdrawal cooldown in seconds."
            ],
            "type": "u64"
          },
          {
            "name": "maxObservationAgeSeconds",
            "docs": [
              "Maximum age (seconds) of the oldest observation before the feed is",
              "considered stale. When stale, `push_apr_observation` preserves the",
              "last known `base_apr_bps` instead of annualizing over a too-old window."
            ],
            "type": "u64"
          },
          {
            "name": "paused",
            "docs": [
              "Bitflag pause state (0x01=senior deposits, 0x02=junior deposits,",
              "0x04=senior withdrawals, 0x08=junior withdrawals)."
            ],
            "type": "u8"
          },
          {
            "name": "padding2",
            "docs": [
              "Padding to align `next_receipt_id` to 8-byte boundary."
            ],
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "nextReceiptId",
            "docs": [
              "Auto-incrementing receipt counter."
            ],
            "type": "u64"
          },
          {
            "name": "trancheConfigBump",
            "docs": [
              "Bump for `tranche_config` PDA."
            ],
            "type": "u8"
          },
          {
            "name": "trancheAuthorityBump",
            "docs": [
              "Bump for `tranche_authority` PDA (CPI signer)."
            ],
            "type": "u8"
          },
          {
            "name": "seniorMintBump",
            "docs": [
              "Bump for `senior_mint` PDA."
            ],
            "type": "u8"
          },
          {
            "name": "juniorMintBump",
            "docs": [
              "Bump for `junior_mint` PDA."
            ],
            "type": "u8"
          },
          {
            "name": "aprFeedBump",
            "docs": [
              "Bump for `apr_feed` PDA."
            ],
            "type": "u8"
          },
          {
            "name": "padding3",
            "docs": [
              "Padding after bumps."
            ],
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future expansion."
            ],
            "type": {
              "array": [
                "u8",
                248
              ]
            }
          }
        ]
      }
    },
    {
      "name": "trancheConfigField",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "aprFloorBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "riskXBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "riskYBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "riskK",
            "fields": [
              {
                "array": [
                  "u8",
                  16
                ]
              }
            ]
          },
          {
            "name": "reserveFeeBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "seniorExitFeeBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "juniorExitFeeBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "minJrSrRatioBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "minCoverageForMintBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "minCoverageBufferBps",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "seniorCooldownSeconds",
            "fields": [
              "u64"
            ]
          },
          {
            "name": "juniorCooldownSeconds",
            "fields": [
              "u64"
            ]
          }
        ]
      }
    },
    {
      "name": "updateTrancheConfigEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "fieldIndex",
            "type": "u8"
          },
          {
            "name": "slot",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "vault",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "name",
            "docs": [
              "The vault's name."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "description",
            "docs": [
              "A description or summary for this vault."
            ],
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "asset",
            "docs": [
              "The vault’s main asset configuration (inline nested struct)."
            ],
            "type": {
              "defined": {
                "name": "vaultAsset"
              }
            }
          },
          {
            "name": "lp",
            "docs": [
              "The vault’s LP (share) configuration (inline nested struct)."
            ],
            "type": {
              "defined": {
                "name": "vaultLp"
              }
            }
          },
          {
            "name": "pendingAdmin",
            "docs": [
              "The pending admin for admin transfer."
            ],
            "type": "pubkey"
          },
          {
            "name": "manager",
            "docs": [
              "The manager of this vault (has certain permissions)."
            ],
            "type": "pubkey"
          },
          {
            "name": "admin",
            "docs": [
              "The admin of this vault (broader or fallback permissions)."
            ],
            "type": "pubkey"
          },
          {
            "name": "vaultConfiguration",
            "docs": [
              "The vault fee, cap, and locked profit degradation duration configuration (inline nested struct)."
            ],
            "type": {
              "defined": {
                "name": "vaultConfiguration"
              }
            }
          },
          {
            "name": "feeConfiguration",
            "docs": [
              "The vault fee and cap configuration (inline nested struct)."
            ],
            "type": {
              "defined": {
                "name": "feeConfiguration"
              }
            }
          },
          {
            "name": "feeUpdate",
            "docs": [
              "The fee update state of the vault."
            ],
            "type": {
              "defined": {
                "name": "feeUpdate"
              }
            }
          },
          {
            "name": "feeState",
            "docs": [
              "The fee state of the vault."
            ],
            "type": {
              "defined": {
                "name": "feeState"
              }
            }
          },
          {
            "name": "deadWeight",
            "docs": [
              "The accumulated dead weight of the vault."
            ],
            "type": "u64"
          },
          {
            "name": "highWaterMark",
            "type": {
              "defined": {
                "name": "highWaterMark"
              }
            }
          },
          {
            "name": "lastUpdatedTs",
            "docs": [
              "The last time (Unix timestamp) this vault data was updated."
            ],
            "type": "u64"
          },
          {
            "name": "version",
            "docs": [
              "The version of the vault."
            ],
            "type": "u8"
          },
          {
            "name": "padding0",
            "docs": [
              "padding to align future 8-byte fields on 8-byte boundaries."
            ],
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "lockedProfitState",
            "docs": [
              "The locked profit state of the vault."
            ],
            "type": {
              "defined": {
                "name": "lockedProfitState"
              }
            }
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": {
              "array": [
                "u8",
                240
              ]
            }
          }
        ]
      }
    },
    {
      "name": "vaultAsset",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The mint for the vault’s main asset."
            ],
            "type": "pubkey"
          },
          {
            "name": "idleAta",
            "docs": [
              "The “idle” token account holding un-invested assets."
            ],
            "type": "pubkey"
          },
          {
            "name": "totalValue",
            "docs": [
              "The total amount of this asset currently in the vault."
            ],
            "type": "u64"
          },
          {
            "name": "idleAtaAuthBump",
            "docs": [
              "The bump for the vault asset mint."
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": {
              "array": [
                "u8",
                95
              ]
            }
          }
        ]
      }
    },
    {
      "name": "vaultConfiguration",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "maxCap",
            "docs": [
              "The maximum total amount allowed in the vault."
            ],
            "type": "u64"
          },
          {
            "name": "startAtTs",
            "docs": [
              "active from timestamp"
            ],
            "type": "u64"
          },
          {
            "name": "lockedProfitDegradationDuration",
            "docs": [
              "The locked profit degradation duration."
            ],
            "type": "u64"
          },
          {
            "name": "withdrawalWaitingPeriod",
            "docs": [
              "The waiting period for a withdrawal. prec: seconds"
            ],
            "type": "u64"
          },
          {
            "name": "disabledOperations",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": "u16"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": {
              "array": [
                "u8",
                46
              ]
            }
          }
        ]
      }
    },
    {
      "name": "vaultLp",
      "serialization": "bytemuckunsafe",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The LP mint (e.g., representing shares in this vault)."
            ],
            "type": "pubkey"
          },
          {
            "name": "mintBump",
            "docs": [
              "The bump for the vault LP mint."
            ],
            "type": "u8"
          },
          {
            "name": "mintAuthBump",
            "docs": [
              "The bump for the vault LP mint authority."
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved bytes for future use."
            ],
            "type": {
              "array": [
                "u8",
                30
              ]
            }
          }
        ]
      }
    },
    {
      "name": "withdrawReceipt",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "trancheConfig",
            "docs": [
              "Parent TrancheConfig."
            ],
            "type": "pubkey"
          },
          {
            "name": "user",
            "docs": [
              "Withdrawing user."
            ],
            "type": "pubkey"
          },
          {
            "name": "receiptId",
            "docs": [
              "Unique, auto-incremented receipt ID."
            ],
            "type": "u64"
          },
          {
            "name": "isSenior",
            "docs": [
              "`true` for senior tranche, `false` for junior."
            ],
            "type": "bool"
          },
          {
            "name": "trancheTokensEscrowed",
            "docs": [
              "Tranche tokens held in escrow."
            ],
            "type": "u64"
          },
          {
            "name": "assetAmountAtRequest",
            "docs": [
              "NAV snapshot at request time (Decimal bits) — value cap for cooldown gaming prevention."
            ],
            "type": "u128"
          },
          {
            "name": "withdrawableFromTs",
            "docs": [
              "Cooldown expiry timestamp."
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump."
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "withdrawReserveEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "amountWithdrawn",
            "type": "u64"
          },
          {
            "name": "reserveNavAfter",
            "type": "u128"
          },
          {
            "name": "totalNavAfter",
            "type": "u128"
          }
        ]
      }
    }
  ]
};
