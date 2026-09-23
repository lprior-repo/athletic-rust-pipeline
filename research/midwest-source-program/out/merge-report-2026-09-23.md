# merge-coaches report

Output: `research/midwest-source-program/data/coach-contacts.csv`

| state | rows | kept | duplicates | rejected |
|---|---|---|---|---|
| AZ | 171 | 171 | 0 | 0 |
| CA | 152 | 152 | 0 | 0 |
| CO | 200 | 200 | 0 | 0 |
| DC | 40 | 24 | 0 | 16 |
| FL | 4 | 4 | 0 | 0 |
| GA | 42 | 40 | 2 | 0 |
| IA | 82 | 82 | 0 | 0 |
| IL | 183 | 183 | 0 | 0 |
| IN | 119 | 119 | 0 | 0 |
| KS | 556 | 556 | 0 | 0 |
| MI | 93 | 86 | 7 | 0 |
| MN | 1180 | 389 | 791 | 0 |
| MO | 60 | 60 | 0 | 0 |
| ND | 180 | 150 | 15 | 15 |
| NE | 100 | 100 | 0 | 0 |
| NJ | 86 | 69 | 17 | 0 |
| OH | 76 | 76 | 0 | 0 |
| OR | 193 | 193 | 0 | 0 |
| TN | 153 | 153 | 0 | 0 |
| TX | 66 | 66 | 0 | 0 |
| UT | 176 | 176 | 0 | 0 |
| WI | 11069 | 3200 | 7869 | 0 |

Total kept: 6249 unique rows

## Rejections

| state | line | school | coach | role | reason | source |
|---|---|---|---|---|---|---|
| DC | 6 | Kipp DC: Legacy College Prep |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 7 | Digital Pioneers Academy |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 12 | Girls Global Academy |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 13 | KIPP DC: College Prep |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 14 | Bard High School Early College DC |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 15 | St. John's College High School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 16 | Model Secondary School for the Deaf |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 23 | Washington International School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 24 | Capital City Public Charter School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 25 | Cardozo High School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 30 | Eastern High School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 37 | Georgetown Day School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 38 | Maret School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 39 | The Lab School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 40 | St. Albans School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| DC | 41 | The Field School |  |  | role is neither a coach nor a director label: "" | https://www.dcsaasports.com/school-directory |
| ND | 143 | West Fargo Sheyenne High School |  | Superintendent | role is neither a coach nor a director label: "Superintendent" | https://ndhsaa.com/schools/1045/west-fargo-sheyenne |
| ND | 144 | West Fargo Sheyenne High School |  | Superintendent | role is neither a coach nor a director label: "Superintendent" | https://ndhsaa.com/schools/1045/west-fargo-sheyenne |
| ND | 145 | West Fargo Sheyenne High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/1045/west-fargo-sheyenne |
| ND | 146 | West Fargo Sheyenne High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/1045/west-fargo-sheyenne |
| ND | 153 | West Fargo Sheyenne High School |  | Business Manager | role is neither a coach nor a director label: "Business Manager" | https://ndhsaa.com/schools/1045/west-fargo-sheyenne |
| ND | 155 | Bismarck Legacy High School |  | Superintendent | role is neither a coach nor a director label: "Superintendent" | https://ndhsaa.com/schools/1131/bismarck-legacy |
| ND | 156 | Bismarck Legacy High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/1131/bismarck-legacy |
| ND | 157 | Bismarck Legacy High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/1131/bismarck-legacy |
| ND | 161 | Bismarck Legacy High School |  | Business Manager | role is neither a coach nor a director label: "Business Manager" | https://ndhsaa.com/schools/1131/bismarck-legacy |
| ND | 162 | West Fargo Horace High School |  | Superintendent | role is neither a coach nor a director label: "Superintendent" | https://ndhsaa.com/schools/1281/west-fargo-horace |
| ND | 170 | Fargo North High School |  | Superintendent | role is neither a coach nor a director label: "Superintendent" | https://ndhsaa.com/schools/24/fargo-north |
| ND | 171 | Fargo North High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/24/fargo-north |
| ND | 176 | Grafton High School |  | Superintendent | role is neither a coach nor a director label: "Superintendent" | https://ndhsaa.com/schools/31/grafton |
| ND | 177 | Grafton High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/31/grafton |
| ND | 178 | Grafton High School |  | Principal | role is neither a coach nor a director label: "Principal" | https://ndhsaa.com/schools/31/grafton |
